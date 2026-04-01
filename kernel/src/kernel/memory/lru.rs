/// LRU (Least Recently Used) page reclaim framework.
///
/// Maintains an approximate-LRU list of reclaimable page frames.
/// Under memory pressure the reclaim engine evicts the oldest pages,
/// optionally writing dirty ones back through the writeback subsystem
/// before freeing the underlying frame.
///
/// ## Configuration
///
/// | Config key                  | Default | Description                         |
/// |-----------------------------|---------|-------------------------------------|
/// | `lru_high_watermark_pct`    | 90      | Start reclaim above this % usage    |
/// | `lru_low_watermark_pct`     | 75      | Stop reclaim below this % usage     |
/// | `lru_batch_size`            | 32      | Frames to scan per reclaim pass     |
/// | `lru_second_chance`         | true    | Use clock/second-chance algorithm   |
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ─── Telemetry ───────────────────────────────────────────────────────

static RECLAIM_SCANS: AtomicU64 = AtomicU64::new(0);
static RECLAIM_EVICTED: AtomicU64 = AtomicU64::new(0);
static RECLAIM_DIRTY_WRITEBACK: AtomicU64 = AtomicU64::new(0);
static RECLAIM_SKIPPED_PINNED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct ReclaimStats {
    pub scans: u64,
    pub evicted: u64,
    pub dirty_writeback: u64,
    pub skipped_pinned: u64,
}

pub fn reclaim_stats() -> ReclaimStats {
    ReclaimStats {
        scans: RECLAIM_SCANS.load(Ordering::Relaxed),
        evicted: RECLAIM_EVICTED.load(Ordering::Relaxed),
        dirty_writeback: RECLAIM_DIRTY_WRITEBACK.load(Ordering::Relaxed),
        skipped_pinned: RECLAIM_SKIPPED_PINNED.load(Ordering::Relaxed),
    }
}

// ─── Page Flags ──────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Per-page metadata flags tracked by the LRU subsystem.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u8 {
        /// Page has been accessed since last scan (clock bit).
        const ACCESSED  = 0b0000_0001;
        /// Page contains dirty data that must be written back before eviction.
        const DIRTY     = 0b0000_0010;
        /// Page is pinned (kernel, DMA, mlock) and must not be evicted.
        const PINNED    = 0b0000_0100;
        /// Page is on the active list (recently promoted from inactive).
        const ACTIVE    = 0b0000_1000;
        /// Page is mapped as anonymous (no file backing).
        const ANONYMOUS = 0b0001_0000;
        /// Page is part of a slab cache.
        const SLAB      = 0b0010_0000;
        /// Page is part of a swap slot (already paged out).
        const SWAPPED   = 0b0100_0000;
    }
}

// ─── LRU Page Entry ──────────────────────────────────────────────────

/// Metadata for a single page tracked by the LRU engine.
#[derive(Debug, Clone, Copy)]
pub struct LruPage {
    /// Physical frame address (page-aligned).
    pub frame_addr: usize,
    /// Owning inode (0 = anonymous page).
    pub inode: u64,
    /// Offset within the inode (in pages).
    pub offset: u64,
    /// Page flags.
    pub flags: PageFlags,
    /// NUMA node this page resides on.
    pub numa_node: u8,
}

// ─── Configuration ───────────────────────────────────────────────────

/// Tunable reclaim parameters.
#[derive(Debug, Clone, Copy)]
pub struct ReclaimConfig {
    /// Start reclaim when memory usage exceeds this % of total.
    pub high_watermark_pct: u8,
    /// Stop reclaim when usage drops below this % of total.
    pub low_watermark_pct: u8,
    /// Number of pages to scan per reclaim pass.
    pub batch_size: usize,
    /// Use second-chance (clock) algorithm: clear ACCESSED bit on first
    /// encounter, evict only if the bit is still clear on second pass.
    pub second_chance: bool,
    /// Maximum dirty pages to writeback per reclaim pass.
    pub max_dirty_writeback: usize,
}

impl Default for ReclaimConfig {
    fn default() -> Self {
        Self {
            high_watermark_pct: 90,
            low_watermark_pct: 75,
            batch_size: 32,
            second_chance: true,
            max_dirty_writeback: 16,
        }
    }
}

// ─── LRU Lists ───────────────────────────────────────────────────────

/// The core LRU reclaim engine.
///
/// Maintains two lists (active / inactive) following the Linux 2-list LRU
/// model.  Pages start on the inactive list; if accessed they are promoted
/// to active.  Reclaim scans the inactive list first.
pub struct LruReclaimer {
    /// Inactive list — candidates for immediate eviction.
    inactive: VecDeque<LruPage>,
    /// Active list — recently accessed; protected from immediate eviction.
    active: VecDeque<LruPage>,
    /// Runtime configuration.
    config: ReclaimConfig,
}

impl LruReclaimer {
    pub fn new(config: ReclaimConfig) -> Self {
        Self {
            inactive: VecDeque::with_capacity(4096),
            active: VecDeque::with_capacity(4096),
            config,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(current_effective_reclaim_config(ReclaimConfig::default()))
    }

    /// Register a new page on the inactive list.
    pub fn add_page(&mut self, page: LruPage) {
        self.inactive.push_back(page);
    }

    /// Mark a page as accessed.  If it is on the inactive list it will be
    /// promoted to the active list on the next scan pass.
    pub fn mark_accessed(&mut self, frame_addr: usize) {
        // Check inactive list first (most common path).
        for p in self.inactive.iter_mut() {
            if p.frame_addr == frame_addr {
                p.flags |= PageFlags::ACCESSED;
                return;
            }
        }
        for p in self.active.iter_mut() {
            if p.frame_addr == frame_addr {
                p.flags |= PageFlags::ACCESSED;
                return;
            }
        }
    }

    /// Mark a page as dirty (needs writeback before eviction).
    pub fn mark_dirty(&mut self, frame_addr: usize) {
        for p in self.inactive.iter_mut().chain(self.active.iter_mut()) {
            if p.frame_addr == frame_addr {
                p.flags |= PageFlags::DIRTY;
                return;
            }
        }
    }

    /// Pin a page (prevent eviction — used for mlock, DMA).
    pub fn pin_page(&mut self, frame_addr: usize) {
        for p in self.inactive.iter_mut().chain(self.active.iter_mut()) {
            if p.frame_addr == frame_addr {
                p.flags |= PageFlags::PINNED;
                return;
            }
        }
    }

    /// Unpin a page.
    pub fn unpin_page(&mut self, frame_addr: usize) {
        for p in self.inactive.iter_mut().chain(self.active.iter_mut()) {
            if p.frame_addr == frame_addr {
                p.flags.remove(PageFlags::PINNED);
                return;
            }
        }
    }

    /// Remove a page from the LRU entirely (e.g. explicit free).
    pub fn remove_page(&mut self, frame_addr: usize) -> Option<LruPage> {
        if let Some(pos) = self
            .inactive
            .iter()
            .position(|p| p.frame_addr == frame_addr)
        {
            return self.inactive.remove(pos);
        }
        if let Some(pos) = self.active.iter().position(|p| p.frame_addr == frame_addr) {
            return self.active.remove(pos);
        }
        None
    }

    /// Total pages tracked (active + inactive).
    pub fn tracked_count(&self) -> usize {
        self.active.len() + self.inactive.len()
    }

    /// Returns true if memory pressure exceeds the high watermark.
    pub fn should_reclaim(&self, total_pages: usize) -> bool {
        if total_pages == 0 {
            return false;
        }
        let used_pct = (self.tracked_count() * 100) / total_pages;
        used_pct >= self.config.high_watermark_pct as usize
    }

    /// Run one reclaim pass.  Returns the list of evicted frame addresses.
    ///
    /// The caller is responsible for:
    /// - Flushing dirty pages (if `dirty_pages` is non-empty) through the
    ///   writeback subsystem *before* freeing the frame.
    /// - Actually freeing the physical frames via the page allocator.
    pub fn reclaim_pass(&mut self) -> ReclaimResult {
        RECLAIM_SCANS.fetch_add(1, Ordering::Relaxed);

        let mut result = ReclaimResult {
            evicted: Vec::new(),
            dirty_writeback: Vec::new(),
        };

        // Phase 1: Promote accessed pages from inactive → active.
        let mut i = 0;
        while i < self.inactive.len() {
            let page = &mut self.inactive[i];
            if page.flags.contains(PageFlags::ACCESSED) && self.config.second_chance {
                page.flags.remove(PageFlags::ACCESSED);
                page.flags.insert(PageFlags::ACTIVE);
                let promoted = self.inactive.remove(i).unwrap();
                self.active.push_back(promoted);
                // Don't increment i — next element shifted into position.
            } else {
                i += 1;
            }
        }

        // Phase 2: Demote cold pages from active → inactive.
        let demote_target = self.active.len().min(self.config.batch_size / 2);
        let mut demoted = 0;
        let mut j = 0;
        while j < self.active.len() && demoted < demote_target {
            let page = &mut self.active[j];
            if page.flags.contains(PageFlags::ACCESSED) {
                // Still hot — give it another chance.
                page.flags.remove(PageFlags::ACCESSED);
                j += 1;
            } else {
                page.flags.remove(PageFlags::ACTIVE);
                let cold = self.active.remove(j).unwrap();
                self.inactive.push_back(cold);
                demoted += 1;
            }
        }

        // Phase 3: Evict from the inactive list (FIFO order = LRU).
        let mut scanned = 0;
        let mut dirty_wb = 0;
        while scanned < self.config.batch_size && !self.inactive.is_empty() {
            let page = match self.inactive.front() {
                Some(p) => *p,
                None => break,
            };

            if page.flags.contains(PageFlags::PINNED) {
                // Move pinned page to the back — it can never be evicted.
                let pinned = self.inactive.pop_front().unwrap();
                self.inactive.push_back(pinned);
                RECLAIM_SKIPPED_PINNED.fetch_add(1, Ordering::Relaxed);
                scanned += 1;
                continue;
            }

            if page.flags.contains(PageFlags::DIRTY) {
                if dirty_wb < self.config.max_dirty_writeback {
                    let dirty = self.inactive.pop_front().unwrap();
                    result.dirty_writeback.push(dirty);
                    RECLAIM_DIRTY_WRITEBACK.fetch_add(1, Ordering::Relaxed);
                    dirty_wb += 1;
                } else {
                    // Too many dirty pages already queued — skip.
                    let skip = self.inactive.pop_front().unwrap();
                    self.inactive.push_back(skip);
                }
                scanned += 1;
                continue;
            }

            // Clean, unpinned page → evict.
            let evicted = self.inactive.pop_front().unwrap();
            result.evicted.push(evicted);
            RECLAIM_EVICTED.fetch_add(1, Ordering::Relaxed);
            scanned += 1;
        }

        result
    }

    /// Update configuration at runtime.
    pub fn set_config(&mut self, config: ReclaimConfig) {
        self.config = config;
    }

    /// Current configuration.
    pub fn config(&self) -> &ReclaimConfig {
        &self.config
    }

    /// Number of pages on the inactive list.
    pub fn inactive_count(&self) -> usize {
        self.inactive.len()
    }

    /// Number of pages on the active list.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[inline(always)]
fn governor_adjusted_watermark_pct(pct: u8, latency_bias: &'static str) -> u8 {
    crate::kernel::virt_bias::adjust_pct_u8(pct, latency_bias, 5)
}

#[inline(always)]
fn governor_adjusted_batch_size(batch_size: usize, latency_bias: &'static str) -> usize {
    crate::kernel::virt_bias::adjust_budget_usize(batch_size, latency_bias)
}

#[inline(always)]
fn reclaim_config_with_bias(config: ReclaimConfig, latency_bias: &'static str) -> ReclaimConfig {
    ReclaimConfig {
        high_watermark_pct: governor_adjusted_watermark_pct(
            config.high_watermark_pct,
            latency_bias,
        ),
        low_watermark_pct: governor_adjusted_watermark_pct(config.low_watermark_pct, latency_bias),
        batch_size: governor_adjusted_batch_size(config.batch_size, latency_bias),
        second_chance: config.second_chance,
        max_dirty_writeback: governor_adjusted_batch_size(config.max_dirty_writeback, latency_bias),
    }
}

#[inline(always)]
fn current_effective_reclaim_config(config: ReclaimConfig) -> ReclaimConfig {
    reclaim_config_with_bias(config, crate::kernel::virt_bias::current_latency_bias())
}

/// Result of a single reclaim pass.
#[derive(Debug)]
pub struct ReclaimResult {
    /// Clean pages that have been evicted — caller must free the frames.
    pub evicted: Vec<LruPage>,
    /// Dirty pages that need writeback before the frame can be freed.
    /// Caller should flush these through the writeback engine and then
    /// call `reclaim_pass` again (they won't be dirty anymore).
    pub dirty_writeback: Vec<LruPage>,
}

#[cfg(test)]
#[path = "lru/tests.rs"]
mod tests;
