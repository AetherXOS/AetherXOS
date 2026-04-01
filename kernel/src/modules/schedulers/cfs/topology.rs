use super::cfs_support::migration_cost as compute_migration_cost;

/// CPU scheduling domain level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedDomainLevel {
    /// SMT siblings (hyperthreads sharing a core).
    Smt,
    /// Cores sharing an L2/L3 cache (same die).
    Llc,
    /// All cores on the same NUMA node.
    NumaNode,
    /// Cross-NUMA (remote nodes).
    Remote,
}

/// Describes one CPU in the topology.
#[derive(Debug, Clone, Copy)]
pub struct CpuTopologyEntry {
    pub cpu_id: u32,
    pub core_id: u32,
    pub numa_node: u8,
    pub llc_id: u16,
    /// Whether this CPU is currently idle.
    pub idle: bool,
    /// Load on this CPU (runqueue length or weighted load).
    pub load: u64,
}

/// CPU topology map for the scheduler.
pub struct CpuTopology {
    cpus: alloc::vec::Vec<CpuTopologyEntry>,
}

impl CpuTopology {
    pub fn new() -> Self {
        Self {
            cpus: alloc::vec::Vec::new(),
        }
    }

    /// Register a CPU in the topology.
    pub fn add_cpu(&mut self, entry: CpuTopologyEntry) {
        self.cpus.push(entry);
    }

    /// Total CPUs.
    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    /// Update load for a CPU.
    pub fn set_load(&mut self, cpu_id: u32, load: u64) {
        for c in &mut self.cpus {
            if c.cpu_id == cpu_id {
                c.load = load;
                return;
            }
        }
    }

    /// Mark a CPU idle/busy.
    pub fn set_idle(&mut self, cpu_id: u32, idle: bool) {
        for c in &mut self.cpus {
            if c.cpu_id == cpu_id {
                c.idle = idle;
                return;
            }
        }
    }

    /// Find an idle CPU on the preferred NUMA node.
    /// Falls back through the topology hierarchy:
    /// 1. Same LLC group (cheapest migration)
    /// 2. Same NUMA node
    /// 3. Nearest NUMA node
    /// 4. Any idle CPU
    pub fn find_idle_cpu(&self, preferred_node: u8, preferred_llc: u16) -> Option<u32> {
        // Level 1: Same LLC, idle
        for c in &self.cpus {
            if c.idle && c.llc_id == preferred_llc {
                return Some(c.cpu_id);
            }
        }
        // Level 2: Same NUMA node, idle
        for c in &self.cpus {
            if c.idle && c.numa_node == preferred_node {
                return Some(c.cpu_id);
            }
        }
        // Level 3: Any idle CPU (sort by node distance in production)
        for c in &self.cpus {
            if c.idle {
                return Some(c.cpu_id);
            }
        }
        None
    }

    /// Find the least loaded CPU on the preferred NUMA node.
    pub fn find_least_loaded_cpu(&self, preferred_node: u8) -> Option<u32> {
        let mut best: Option<(u32, u64)> = None;
        // Prefer same node.
        for c in &self.cpus {
            if c.numa_node == preferred_node {
                match best {
                    None => best = Some((c.cpu_id, c.load)),
                    Some((_, bl)) if c.load < bl => best = Some((c.cpu_id, c.load)),
                    _ => {}
                }
            }
        }
        if best.is_some() {
            return best.map(|(id, _)| id);
        }
        // Fallback: any CPU.
        for c in &self.cpus {
            match best {
                None => best = Some((c.cpu_id, c.load)),
                Some((_, bl)) if c.load < bl => best = Some((c.cpu_id, c.load)),
                _ => {}
            }
        }
        best.map(|(id, _)| id)
    }

    /// Get NUMA node for a CPU.
    pub fn cpu_node(&self, cpu_id: u32) -> Option<u8> {
        self.cpus
            .iter()
            .find(|c| c.cpu_id == cpu_id)
            .map(|c| c.numa_node)
    }

    /// Get LLC id for a CPU.
    pub fn cpu_llc(&self, cpu_id: u32) -> Option<u16> {
        self.cpus
            .iter()
            .find(|c| c.cpu_id == cpu_id)
            .map(|c| c.llc_id)
    }

    /// List all CPUs on a NUMA node.
    pub fn cpus_on_node(&self, node: u8) -> alloc::vec::Vec<u32> {
        self.cpus
            .iter()
            .filter(|c| c.numa_node == node)
            .map(|c| c.cpu_id)
            .collect()
    }

    /// Compute distance penalty between two CPUs based on topology.
    pub fn migration_cost(&self, from_cpu: u32, to_cpu: u32) -> u64 {
        let from = self.cpus.iter().find(|c| c.cpu_id == from_cpu);
        let to = self.cpus.iter().find(|c| c.cpu_id == to_cpu);
        match (from, to) {
            (Some(f), Some(t)) => compute_migration_cost(
                Some(f.core_id),
                Some(t.core_id),
                Some(f.llc_id),
                Some(t.llc_id),
                Some(f.numa_node),
                Some(t.numa_node),
            ),
            _ => u64::MAX,
        }
    }
}
