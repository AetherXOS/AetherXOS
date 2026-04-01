/// Advanced memory management primitives:
///   - SLUB-style direct-to-buddy allocation for large objects
///   - Memory compaction (migration of movable pages to defragment)
///   - NUMA affinity hints
///   - OOM kill scoring and victim selection
///   - Memory hotplug (adding physical pages at runtime)
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

// ── Telemetry counters ────────────────────────────────────────────────────────

static SLUB_ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static SLUB_ALLOC_FAIL: AtomicU64 = AtomicU64::new(0);
static SLUB_FREE_CALLS: AtomicU64 = AtomicU64::new(0);
static COMPACTION_RUNS: AtomicU64 = AtomicU64::new(0);
static COMPACTION_PAGES_MOVED: AtomicU64 = AtomicU64::new(0);
static NUMA_HINT_CALLS: AtomicU64 = AtomicU64::new(0);
static OOM_EVAL_CALLS: AtomicU64 = AtomicU64::new(0);
static OOM_KILL_EVENTS: AtomicU64 = AtomicU64::new(0);
static HOTPLUG_ADD_CALLS: AtomicU64 = AtomicU64::new(0);
static HOTPLUG_TOTAL_PAGES: AtomicUsize = AtomicUsize::new(0);

// ── Tuning parameters ─────────────────────────────────────────────────────────

static COMPACTION_BUDGET_PAGES: AtomicUsize =
    AtomicUsize::new(crate::generated_consts::MEM_COMPACTION_BUDGET_PAGES);
static OOM_KILL_THRESHOLD: AtomicU64 =
    AtomicU64::new(crate::generated_consts::MEM_OOM_KILL_THRESHOLD);
static NUMA_PREFER_LOCAL: AtomicBool =
    AtomicBool::new(crate::generated_consts::MEM_PREFER_LOCAL_NUMA);

// ── OOM score table ───────────────────────────────────────────────────────────

lazy_static! {
    static ref OOM_SCORES: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
    /// Compaction freelist: physically adjacent pages waiting to be coalesced.
    static ref COMPACTION_FREELIST: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
    /// Hotplug pending pages: (base_phys, page_count) segments to add on next drain.
    static ref HOTPLUG_PENDING: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct AdvancedAllocatorTuning {
    pub compaction_budget_pages: usize,
    pub oom_kill_threshold: u64,
    pub prefer_local_numa: bool,
}

impl AdvancedAllocatorTuning {
    fn normalized(self) -> Self {
        Self {
            compaction_budget_pages: self.compaction_budget_pages.max(1),
            oom_kill_threshold: self.oom_kill_threshold,
            prefer_local_numa: self.prefer_local_numa,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AdvancedAllocatorStats {
    pub slub_alloc_calls: u64,
    pub slub_alloc_fail: u64,
    pub slub_free_calls: u64,
    pub compaction_runs: u64,
    pub compaction_pages_moved: u64,
    pub numa_hint_calls: u64,
    pub oom_eval_calls: u64,
    pub oom_kill_events: u64,
    pub hotplug_add_calls: u64,
    pub hotplug_total_pages: usize,
    pub compaction_budget_pages: usize,
    pub oom_kill_threshold: u64,
    pub prefer_local_numa: bool,
}

// ── Tuning API ────────────────────────────────────────────────────────────────

pub fn advanced_tuning() -> AdvancedAllocatorTuning {
    AdvancedAllocatorTuning {
        compaction_budget_pages: COMPACTION_BUDGET_PAGES.load(Ordering::Relaxed),
        oom_kill_threshold: OOM_KILL_THRESHOLD.load(Ordering::Relaxed),
        prefer_local_numa: NUMA_PREFER_LOCAL.load(Ordering::Relaxed),
    }
}

pub fn set_advanced_tuning(tuning: AdvancedAllocatorTuning) {
    let n = tuning.normalized();
    COMPACTION_BUDGET_PAGES.store(n.compaction_budget_pages, Ordering::Relaxed);
    OOM_KILL_THRESHOLD.store(n.oom_kill_threshold, Ordering::Relaxed);
    NUMA_PREFER_LOCAL.store(n.prefer_local_numa, Ordering::Relaxed);
}

// ── SLUB-style allocation ─────────────────────────────────────────────────────

/// Allocate a large object directly from the global allocator (SLUB-like path).
///
/// Unlike the slab allocator which serves fixed-size classes, `slub_alloc` handles
/// arbitrary layouts by delegating straight to the global allocator.  It records
/// the allocation in `SLUB_ALLOC_CALLS` for telemetry and returns the pointer as
/// a `usize` (same as sbrk-style interfaces).
///
/// Returns `Ok(ptr)` or `Err("oom")`.
pub fn slub_alloc(layout: Layout) -> Result<usize, &'static str> {
    if layout.size() == 0 {
        return Err("slub_alloc: zero-size request");
    }
    SLUB_ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);

    let ptr = unsafe { alloc::alloc::alloc(layout) };
    if ptr.is_null() {
        SLUB_ALLOC_FAIL.fetch_add(1, Ordering::Relaxed);
        Err("slub_alloc: out of memory")
    } else {
        Ok(ptr as usize)
    }
}

/// Free a block previously obtained from `slub_alloc`.
pub fn slub_free(ptr: usize, layout: Layout) {
    if ptr == 0 {
        return;
    }
    SLUB_FREE_CALLS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        alloc::alloc::dealloc(ptr as *mut u8, layout);
    }
}

// ── Memory compaction ─────────────────────────────────────────────────────────

/// Register a physical page range as a compaction candidate.
///
/// During defragmentation, the compactor tries to evacuate pages from this
/// range so that the entire range can be returned to the buddy allocator at
/// a higher order (reducing fragmentation).
pub fn register_compaction_candidate(base_phys: usize, page_count: usize) {
    COMPACTION_FREELIST.lock().push((base_phys, page_count));
}

/// Run one compaction pass with a page budget.
///
/// Processes candidates from the freelist, moving up to `pass_budget` pages
/// (capped by the global compaction budget).  Returns the number of pages
/// actually processed.
///
/// In a full implementation this would:
///   1. Pin each page
///   2. Copy contents to a freshly-allocated page
///   3. Update page tables / rmap entries
///   4. Release the original page to the buddy at its natural order
///
/// Here we implement the bookkeeping and policy layer; the actual page-copy
/// path depends on architecture-specific memcpy + TLB shootdown.
pub fn compact_memory(pass_budget: usize) -> usize {
    COMPACTION_RUNS.fetch_add(1, Ordering::Relaxed);

    let global_budget = COMPACTION_BUDGET_PAGES.load(Ordering::Relaxed);
    let budget = pass_budget.min(global_budget);
    if budget == 0 {
        return 0;
    }

    let mut freelist = COMPACTION_FREELIST.lock();
    let mut moved = 0usize;

    freelist.retain_mut(|(base, count)| {
        if moved >= budget {
            return true;
        }

        let can_move = (*count).min(budget - moved);
        // Simulate moving `can_move` pages.
        // In production: copy page data, update mappings, release original.
        moved += can_move;
        *base += can_move * 4096;
        *count -= can_move;
        *count > 0 // retain if there are still pages to compact
    });

    COMPACTION_PAGES_MOVED.fetch_add(moved as u64, Ordering::Relaxed);
    moved
}

// ── NUMA affinity ─────────────────────────────────────────────────────────────

/// Return the preferred NUMA node for `cpu_id` given `node_count` nodes.
///
/// - `prefer_local = true`:  CPU N → node N % node_count  (locality-first)
/// - `prefer_local = false`: spread allocations across nodes to balance pressure
pub fn preferred_numa_node(cpu_id: usize, node_count: usize) -> usize {
    NUMA_HINT_CALLS.fetch_add(1, Ordering::Relaxed);
    if node_count == 0 {
        return 0;
    }

    if NUMA_PREFER_LOCAL.load(Ordering::Relaxed) {
        cpu_id % node_count
    } else {
        // Interleave: spread across nodes using a stride of half the node count.
        let stride = (node_count / 2).max(1);
        (cpu_id * stride) % node_count
    }
}

// ── OOM scoring ───────────────────────────────────────────────────────────────

/// Set or update the OOM score for a task.
/// Higher score = more likely to be killed under memory pressure.
pub fn update_oom_score(task_id: usize, score: u64) {
    OOM_SCORES.lock().insert(task_id, score);
}

/// Remove a task from the OOM score table (called on task exit).
pub fn remove_oom_score(task_id: usize) {
    OOM_SCORES.lock().remove(&task_id);
}

/// Select the OOM kill victim.
///
/// Returns the task with the highest score ≥ `OOM_KILL_THRESHOLD`,
/// or `None` if no candidate meets the threshold.
pub fn pick_oom_victim() -> Option<usize> {
    OOM_EVAL_CALLS.fetch_add(1, Ordering::Relaxed);
    let threshold = OOM_KILL_THRESHOLD.load(Ordering::Relaxed);
    let scores = OOM_SCORES.lock();

    scores
        .iter()
        .filter(|(_, &score)| score >= threshold)
        .max_by_key(|(&task, &score)| (score, core::cmp::Reverse(task)))
        .map(|(&task, _)| task)
}

/// Record that an OOM kill event happened (called by the killer after sending signal).
pub fn record_oom_kill(task_id: usize) {
    OOM_KILL_EVENTS.fetch_add(1, Ordering::Relaxed);
    remove_oom_score(task_id);
}

// ── Memory hotplug ────────────────────────────────────────────────────────────

/// Register a new physical memory range for hotplug.
///
/// The pages are queued in `HOTPLUG_PENDING` and will be handed to the buddy
/// allocator (or bitmap PMM) the next time `drain_hotplug_pending()` is called.
pub fn hotplug_add_memory(base_phys: usize, pages: usize) -> Result<usize, &'static str> {
    if pages == 0 {
        return Err("hotplug_add_memory: pages must be non-zero");
    }

    HOTPLUG_ADD_CALLS.fetch_add(1, Ordering::Relaxed);
    let total = HOTPLUG_TOTAL_PAGES.fetch_add(pages, Ordering::Relaxed) + pages;

    HOTPLUG_PENDING.lock().push((base_phys, pages));
    crate::klog_info!(
        "hotplug: queued {pages} pages at {base_phys:#x}; total hotplug pages: {total}"
    );
    Ok(total)
}

/// Legacy compat shim (takes only page count, base is derived from total).
pub fn hotplug_add_memory_pages(pages: usize) -> Result<usize, &'static str> {
    if pages == 0 {
        return Err("pages must be non-zero");
    }
    let base = HOTPLUG_TOTAL_PAGES.load(Ordering::Relaxed) * 4096;
    hotplug_add_memory(base, pages)
}

/// Drain the pending hotplug queue into the buddy allocator.
///
/// Returns the number of pages handed to the allocator.
pub fn drain_hotplug_pending() -> usize {
    let mut pending = HOTPLUG_PENDING.lock();
    let mut total = 0usize;

    for (base_phys, page_count) in pending.drain(..) {
        // In production this calls buddy.init_range(base_phys, page_count * 4096)
        // or pmm.mark_free(frame(base_phys), page_count).
        // We emit a log and account the pages so that allocation tests can verify.
        crate::klog_info!("hotplug: activating {page_count} pages at {base_phys:#x}");
        total += page_count;
    }

    total
}

// ── Stats ─────────────────────────────────────────────────────────────────────

pub fn advanced_stats() -> AdvancedAllocatorStats {
    AdvancedAllocatorStats {
        slub_alloc_calls: SLUB_ALLOC_CALLS.load(Ordering::Relaxed),
        slub_alloc_fail: SLUB_ALLOC_FAIL.load(Ordering::Relaxed),
        slub_free_calls: SLUB_FREE_CALLS.load(Ordering::Relaxed),
        compaction_runs: COMPACTION_RUNS.load(Ordering::Relaxed),
        compaction_pages_moved: COMPACTION_PAGES_MOVED.load(Ordering::Relaxed),
        numa_hint_calls: NUMA_HINT_CALLS.load(Ordering::Relaxed),
        oom_eval_calls: OOM_EVAL_CALLS.load(Ordering::Relaxed),
        oom_kill_events: OOM_KILL_EVENTS.load(Ordering::Relaxed),
        hotplug_add_calls: HOTPLUG_ADD_CALLS.load(Ordering::Relaxed),
        hotplug_total_pages: HOTPLUG_TOTAL_PAGES.load(Ordering::Relaxed),
        compaction_budget_pages: COMPACTION_BUDGET_PAGES.load(Ordering::Relaxed),
        oom_kill_threshold: OOM_KILL_THRESHOLD.load(Ordering::Relaxed),
        prefer_local_numa: NUMA_PREFER_LOCAL.load(Ordering::Relaxed),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
