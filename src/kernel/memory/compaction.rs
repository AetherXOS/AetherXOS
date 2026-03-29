/// Memory compaction — reduces physical fragmentation by migrating pages
/// to create larger contiguous free regions (needed for huge page allocation).
///
/// Strategy: scan from bottom for movable (user) pages, scan from top for
/// free slots, migrate pages and update page tables accordingly.
use alloc::vec::Vec;

/// Represents a page that can be migrated during compaction.
#[derive(Debug, Clone, Copy)]
pub struct MovablePage {
    /// Physical address of the page.
    pub phys_addr: usize,
    /// Owner task/process id (for page table update).
    pub owner_id: usize,
    /// Virtual address this page is mapped at in the owner's address space.
    pub virt_addr: usize,
}

/// Result of a compaction pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompactionResult {
    /// Number of pages successfully migrated.
    pub pages_migrated: usize,
    /// Number of contiguous free regions created or enlarged.
    pub free_regions_improved: usize,
    /// Largest contiguous free region (in pages) after compaction.
    pub largest_free_run: usize,
}

/// Describes a free region in physical address space.
#[derive(Debug, Clone, Copy)]
pub struct FreeRegion {
    pub base: usize,
    pub page_count: usize,
}

/// Zone type for grouping pages by mobility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMobility {
    /// Unmovable (kernel code, page tables, DMA buffers).
    Unmovable,
    /// Movable (user pages, file cache pages).
    Movable,
    /// Reclaimable (slab caches, page cache pages that can be evicted).
    Reclaimable,
}

/// Per-zone tracking for anti-fragmentation.
pub struct Zone {
    /// Zone base physical address.
    pub base: usize,
    /// Total pages in zone.
    pub total_pages: usize,
    /// Free page list (physical addresses).
    pub free_list: Vec<usize>,
    /// Mobility class of this zone.
    pub mobility: PageMobility,
}

impl Zone {
    pub fn new(base: usize, total_pages: usize, mobility: PageMobility) -> Self {
        Self {
            base,
            total_pages,
            free_list: Vec::new(),
            mobility,
        }
    }

    pub fn free_count(&self) -> usize {
        self.free_list.len()
    }

    pub fn alloc(&mut self) -> Option<usize> {
        self.free_list.pop()
    }

    pub fn free(&mut self, addr: usize) {
        self.free_list.push(addr);
    }
}

/// Find contiguous free runs in a sorted free list.
fn find_free_runs(sorted_free: &[usize], page_size: usize) -> Vec<FreeRegion> {
    if sorted_free.is_empty() {
        return Vec::new();
    }
    let mut runs = Vec::new();
    let mut run_base = sorted_free[0];
    let mut run_len: usize = 1;
    for i in 1..sorted_free.len() {
        if sorted_free[i] == sorted_free[i - 1] + page_size {
            run_len += 1;
        } else {
            runs.push(FreeRegion {
                base: run_base,
                page_count: run_len,
            });
            run_base = sorted_free[i];
            run_len = 1;
        }
    }
    runs.push(FreeRegion {
        base: run_base,
        page_count: run_len,
    });
    runs
}

/// Compact a zone by migrating movable pages to consolidate free space.
///
/// `movable_pages` — pages in this zone that can be migrated.
/// `migrate_fn` — callback that performs the actual page copy + page table update.
///   Signature: `(old_phys, new_phys, owner_id, virt_addr) -> bool`
/// `max_migrations` — limit on how many pages to move per pass.
pub fn compact_zone(
    zone: &mut Zone,
    movable_pages: &[MovablePage],
    migrate_fn: &mut dyn FnMut(usize, usize, usize, usize) -> bool,
    max_migrations: usize,
) -> CompactionResult {
    if zone.free_list.is_empty() || movable_pages.is_empty() {
        return CompactionResult::default();
    }
    let effective_max_migrations = current_effective_max_migrations(max_migrations);

    // Sort free list for contiguity analysis.
    zone.free_list.sort_unstable();
    let page_size = 4096usize;

    // Find movable pages in the high address range (top of zone).
    let zone_mid = zone.base + (zone.total_pages / 2) * page_size;
    let mut high_movable: Vec<&MovablePage> = movable_pages
        .iter()
        .filter(|p| p.phys_addr >= zone_mid)
        .collect();
    high_movable.sort_unstable_by_key(|p| core::cmp::Reverse(p.phys_addr));

    // Find free pages in the low address range (bottom of zone).
    let low_free: Vec<usize> = zone
        .free_list
        .iter()
        .copied()
        .filter(|&a| a < zone_mid)
        .collect();

    let mut result = CompactionResult::default();
    let mut low_idx = 0;

    for movable in high_movable.iter() {
        if result.pages_migrated >= effective_max_migrations {
            break;
        }
        if low_idx >= low_free.len() {
            break;
        }
        let new_phys = low_free[low_idx];
        if migrate_fn(
            movable.phys_addr,
            new_phys,
            movable.owner_id,
            movable.virt_addr,
        ) {
            // Remove the used free page from zone's free list.
            if let Some(pos) = zone.free_list.iter().position(|&a| a == new_phys) {
                zone.free_list.swap_remove(pos);
            }
            // The old physical page is now free.
            zone.free_list.push(movable.phys_addr);
            result.pages_migrated += 1;
            low_idx += 1;
        }
    }

    // Re-sort and compute largest free run.
    zone.free_list.sort_unstable();
    let runs = find_free_runs(&zone.free_list, page_size);
    result.largest_free_run = runs.iter().map(|r| r.page_count).max().unwrap_or(0);
    result.free_regions_improved = runs.len();
    result
}

/// Check if a huge page (512 contiguous 4K pages = 2 MiB) can be allocated from the zone.
pub fn can_alloc_huge_page(zone: &Zone) -> bool {
    if zone.free_list.len() < 512 {
        return false;
    }
    let mut sorted = zone.free_list.clone();
    sorted.sort_unstable();
    let runs = find_free_runs(&sorted, 4096);
    runs.iter().any(|r| r.page_count >= 512)
}

/// Try to allocate a huge page (512 contiguous 4K pages) from the zone.
pub fn alloc_huge_page(zone: &mut Zone) -> Option<usize> {
    zone.free_list.sort_unstable();
    let runs = find_free_runs(&zone.free_list, 4096);
    // Find first run with >= 512 pages.
    let run = runs.iter().find(|r| r.page_count >= 512)?;
    let base = run.base;
    // Remove 512 pages starting from base.
    for i in 0..512usize {
        let addr = base + i * 4096;
        if let Some(pos) = zone.free_list.iter().position(|&a| a == addr) {
            zone.free_list.swap_remove(pos);
        }
    }
    Some(base)
}

#[inline(always)]
fn governor_adjusted_max_migrations(max_migrations: usize, latency_bias: &'static str) -> usize {
    crate::kernel::virt_bias::adjust_budget_usize(max_migrations, latency_bias)
}

#[inline(always)]
fn current_effective_max_migrations(max_migrations: usize) -> usize {
    governor_adjusted_max_migrations(
        max_migrations,
        crate::kernel::virt_bias::current_latency_bias(),
    )
}

#[cfg(test)]
#[path = "compaction/tests.rs"]
mod tests;
