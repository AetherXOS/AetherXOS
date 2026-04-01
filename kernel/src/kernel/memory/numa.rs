/// NUMA-aware page allocation layer.
///
/// Sits above the raw page allocator and tracks which NUMA node each physical
/// page range belongs to, enabling node-local allocations and tracking distance
/// between nodes for migration decisions.
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Maximum NUMA nodes supported.
pub const MAX_NUMA_NODES: usize = 8;

/// Distance matrix entry (units match ACPI SLIT values, 10 = local).
pub const NUMA_DISTANCE_LOCAL: u8 = 10;
pub const NUMA_DISTANCE_REMOTE_DEFAULT: u8 = 20;

/// Describes a contiguous physical memory region belonging to a NUMA node.
#[derive(Debug, Clone, Copy)]
pub struct NumaRegion {
    pub node_id: u8,
    pub base: usize,
    /// Number of 4 KiB pages in this region.
    pub page_count: usize,
}

/// Per-node allocation state.
#[derive(Debug)]
struct NodeState {
    /// Free page addresses (stack-based for O(1) alloc/free).
    free_pages: Vec<usize>,
    /// Total pages belonging to this node.
    total_pages: usize,
}

impl NodeState {
    fn new() -> Self {
        Self {
            free_pages: Vec::new(),
            total_pages: 0,
        }
    }

    fn alloc(&mut self) -> Option<usize> {
        self.free_pages.pop()
    }

    fn free(&mut self, addr: usize) {
        self.free_pages.push(addr);
    }

    fn free_count(&self) -> usize {
        self.free_pages.len()
    }
}

/// NUMA-aware page allocator.
pub struct NumaAllocator {
    nodes: [Option<NodeState>; MAX_NUMA_NODES],
    /// Map: page address >> 12 → node_id for reverse lookup.
    page_to_node: BTreeMap<usize, u8>,
    /// Distance matrix (symmetric). `distance[a][b]` = distance from node a to b.
    distance: [[u8; MAX_NUMA_NODES]; MAX_NUMA_NODES],
    /// Number of active nodes.
    active_nodes: usize,
}

impl NumaAllocator {
    /// Create a new NUMA allocator. Initially no nodes are registered.
    pub fn new() -> Self {
        let mut distance = [[NUMA_DISTANCE_REMOTE_DEFAULT; MAX_NUMA_NODES]; MAX_NUMA_NODES];
        for i in 0..MAX_NUMA_NODES {
            distance[i][i] = NUMA_DISTANCE_LOCAL;
        }
        Self {
            nodes: Default::default(),
            page_to_node: BTreeMap::new(),
            distance,
            active_nodes: 0,
        }
    }

    /// Register a NUMA node's physical memory region.
    /// Pages are added to the node's free list.
    pub fn register_region(&mut self, region: NumaRegion) {
        let nid = region.node_id as usize;
        if nid >= MAX_NUMA_NODES {
            return;
        }
        let state = self.nodes[nid].get_or_insert_with(NodeState::new);
        state.total_pages += region.page_count;
        for i in 0..region.page_count {
            let addr = region.base + i * 4096;
            state.free_pages.push(addr);
            self.page_to_node.insert(addr >> 12, region.node_id);
        }
        if self.nodes[nid].is_some() && nid >= self.active_nodes {
            self.active_nodes = nid + 1;
        }
    }

    /// Set distance between two NUMA nodes.
    pub fn set_distance(&mut self, from: u8, to: u8, dist: u8) {
        let f = from as usize;
        let t = to as usize;
        if f < MAX_NUMA_NODES && t < MAX_NUMA_NODES {
            self.distance[f][t] = dist;
            self.distance[t][f] = dist;
        }
    }

    /// Allocate a page from the preferred NUMA node.
    /// Falls back to nearest node with free pages.
    pub fn alloc_page(&mut self, preferred_node: u8) -> Option<(usize, u8)> {
        let pref = preferred_node as usize;
        // Try preferred node first.
        if pref < MAX_NUMA_NODES {
            if let Some(ref mut state) = self.nodes[pref] {
                if let Some(addr) = state.alloc() {
                    return Some((addr, preferred_node));
                }
            }
        }
        // Fallback: try nodes in distance order from preferred.
        let mut candidates: Vec<(u8, usize)> = Vec::new();
        for i in 0..self.active_nodes {
            if i == pref {
                continue;
            }
            let dist = self.distance[pref.min(MAX_NUMA_NODES - 1)][i];
            candidates.push((dist, i));
        }
        candidates.sort_unstable_by_key(|&(d, _)| d);

        for (_, nid) in candidates {
            if let Some(ref mut state) = self.nodes[nid] {
                if let Some(addr) = state.alloc() {
                    return Some((addr, nid as u8));
                }
            }
        }
        None
    }

    /// Free a page. Automatically returns it to the correct node.
    pub fn free_page(&mut self, addr: usize) {
        let key = addr >> 12;
        if let Some(&nid) = self.page_to_node.get(&key) {
            if let Some(ref mut state) = self.nodes[nid as usize] {
                state.free(addr);
            }
        }
    }

    /// Look up which NUMA node owns a page.
    pub fn page_node(&self, addr: usize) -> Option<u8> {
        self.page_to_node.get(&(addr >> 12)).copied()
    }

    /// Distance between two nodes.
    pub fn node_distance(&self, a: u8, b: u8) -> u8 {
        let a = a as usize;
        let b = b as usize;
        if a < MAX_NUMA_NODES && b < MAX_NUMA_NODES {
            self.distance[a][b]
        } else {
            u8::MAX
        }
    }

    /// Per-node free page counts.
    pub fn free_counts(&self) -> Vec<(u8, usize, usize)> {
        let mut out = Vec::new();
        for i in 0..self.active_nodes {
            if let Some(ref state) = self.nodes[i] {
                out.push((i as u8, state.free_count(), state.total_pages));
            }
        }
        out
    }

    /// Total free pages across all nodes.
    pub fn total_free(&self) -> usize {
        let mut total = 0;
        for i in 0..self.active_nodes {
            if let Some(ref s) = self.nodes[i] {
                total += s.free_count();
            }
        }
        total
    }

    /// Total pages across all nodes.
    pub fn total_pages(&self) -> usize {
        let mut total = 0;
        for i in 0..self.active_nodes {
            if let Some(ref s) = self.nodes[i] {
                total += s.total_pages;
            }
        }
        total
    }
}
