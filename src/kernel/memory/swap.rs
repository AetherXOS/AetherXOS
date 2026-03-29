/// Swap subsystem — manages swap areas (block-backed page store).
///
/// Provides a page-granular swap space that the LRU reclaimer can use to
/// page out anonymous memory.  Swap slots are tracked via a bitmap.
///
/// ## Design
///
/// - Multiple swap areas can be registered (e.g. swap partition + swap file).
/// - Priority ordering controls which area is used first.
/// - Swap-in reads the slot and marks it free; swap-out allocates a slot
///   and writes the page data through the block-device abstraction.
///
/// ## Configuration
///
/// | Key                   | Default | Description                      |
/// |-----------------------|---------|----------------------------------|
/// | `swap_enabled`        | false   | Master switch for swap support   |
/// | `swap_max_areas`      | 4       | Maximum concurrent swap areas    |
/// | `swap_readahead`      | 8       | Slots to readahead on swap-in    |
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ─── Telemetry ───────────────────────────────────────────────────────

static SWAP_IN_COUNT: AtomicU64 = AtomicU64::new(0);
static SWAP_OUT_COUNT: AtomicU64 = AtomicU64::new(0);
static SWAP_ALLOC_FAIL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct SwapStats {
    pub swap_in: u64,
    pub swap_out: u64,
    pub alloc_fail: u64,
    pub total_slots: u64,
    pub free_slots: u64,
}

// ─── Swap Slot ───────────────────────────────────────────────────────

/// Opaque handle to a single swap slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SwapSlot {
    /// Index of the swap area that owns this slot.
    pub area_idx: u8,
    /// Slot offset within the swap area (in pages).
    pub offset: u32,
}

// ─── Swap Area ───────────────────────────────────────────────────────

/// A single swap area backed by a block device or file.
#[derive(Debug)]
pub struct SwapArea {
    /// Human-readable name (e.g. "/dev/sda2", "swapfile").
    pub name: String,
    /// Priority (higher = preferred).
    pub priority: i16,
    /// Total number of page-sized slots.
    total_slots: u32,
    /// Bitmap: 1 = used, 0 = free.
    bitmap: Vec<u64>,
    /// Cached count of free slots.
    free_count: u32,
    /// Whether this area is active.
    active: bool,
}

impl SwapArea {
    /// Create a new swap area with `slot_count` pages of capacity.
    pub fn new(name: String, slot_count: u32, priority: i16) -> Self {
        let bitmap_words = ((slot_count as usize) + 63) / 64;
        Self {
            name,
            priority,
            total_slots: slot_count,
            bitmap: vec![0u64; bitmap_words],
            free_count: slot_count,
            active: false,
        }
    }

    /// Activate this swap area.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate this swap area (no new allocations).
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Allocate a free slot. Returns the slot offset or None.
    fn alloc_slot(&mut self) -> Option<u32> {
        if !self.active || self.free_count == 0 {
            return None;
        }
        for (word_idx, word) in self.bitmap.iter_mut().enumerate() {
            if *word != u64::MAX {
                let bit = (!*word).trailing_zeros();
                let slot = (word_idx as u32) * 64 + bit;
                if slot >= self.total_slots {
                    return None;
                }
                *word |= 1u64 << bit;
                self.free_count -= 1;
                return Some(slot);
            }
        }
        None
    }

    /// Free a slot.
    fn free_slot(&mut self, offset: u32) {
        if offset >= self.total_slots {
            return;
        }
        let word_idx = (offset / 64) as usize;
        let bit = offset % 64;
        if word_idx < self.bitmap.len() {
            let was_set = self.bitmap[word_idx] & (1u64 << bit) != 0;
            self.bitmap[word_idx] &= !(1u64 << bit);
            if was_set {
                self.free_count += 1;
            }
        }
    }

    /// Check if a slot is allocated.
    fn is_slot_used(&self, offset: u32) -> bool {
        if offset >= self.total_slots {
            return false;
        }
        let word_idx = (offset / 64) as usize;
        let bit = offset % 64;
        if word_idx < self.bitmap.len() {
            self.bitmap[word_idx] & (1u64 << bit) != 0
        } else {
            false
        }
    }

    pub fn free_count(&self) -> u32 {
        self.free_count
    }

    pub fn total_slots(&self) -> u32 {
        self.total_slots
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

// ─── Swap Manager ────────────────────────────────────────────────────

/// Maximum number of swap areas.
pub const MAX_SWAP_AREAS: usize = 4;

/// Central swap manager.
pub struct SwapManager {
    areas: Vec<SwapArea>,
    /// Whether swap is globally enabled.
    enabled: bool,
    /// Readahead count for swap-in.
    readahead_slots: u32,
}

impl SwapManager {
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            enabled: false,
            readahead_slots: 8,
        }
    }

    /// Enable swap globally.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable swap globally (no new swap-outs; existing swapped pages
    /// stay until swapped in).
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set readahead count.
    pub fn set_readahead(&mut self, slots: u32) {
        self.readahead_slots = slots.max(1);
    }

    /// Register a new swap area.  Returns the area index or None if full.
    pub fn add_area(&mut self, area: SwapArea) -> Option<u8> {
        if self.areas.len() >= MAX_SWAP_AREAS {
            return None;
        }
        let idx = self.areas.len() as u8;
        self.areas.push(area);
        // Re-sort by priority (highest first).
        self.areas.sort_by(|a, b| b.priority.cmp(&a.priority));
        Some(idx)
    }

    /// Activate a swap area by name.
    pub fn activate_area(&mut self, name: &str) -> bool {
        for area in &mut self.areas {
            if area.name == name {
                area.activate();
                return true;
            }
        }
        false
    }

    /// Deactivate a swap area by name.
    pub fn deactivate_area(&mut self, name: &str) -> bool {
        for area in &mut self.areas {
            if area.name == name {
                area.deactivate();
                return true;
            }
        }
        false
    }

    /// Allocate a swap slot (for swap-out).
    /// Tries areas in priority order.
    pub fn alloc_slot(&mut self) -> Option<SwapSlot> {
        if !self.enabled {
            return None;
        }
        for (idx, area) in self.areas.iter_mut().enumerate() {
            if let Some(offset) = area.alloc_slot() {
                SWAP_OUT_COUNT.fetch_add(1, Ordering::Relaxed);
                return Some(SwapSlot {
                    area_idx: idx as u8,
                    offset,
                });
            }
        }
        SWAP_ALLOC_FAIL.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Free a swap slot (after swap-in or process exit).
    pub fn free_slot(&mut self, slot: SwapSlot) {
        let idx = slot.area_idx as usize;
        if idx < self.areas.len() {
            self.areas[idx].free_slot(slot.offset);
            SWAP_IN_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Check if a slot is still allocated.
    pub fn is_slot_used(&self, slot: SwapSlot) -> bool {
        let idx = slot.area_idx as usize;
        if idx < self.areas.len() {
            self.areas[idx].is_slot_used(slot.offset)
        } else {
            false
        }
    }

    /// Get swap statistics.
    pub fn stats(&self) -> SwapStats {
        let mut total: u64 = 0;
        let mut free: u64 = 0;
        for area in &self.areas {
            total += area.total_slots() as u64;
            free += area.free_count() as u64;
        }
        SwapStats {
            swap_in: SWAP_IN_COUNT.load(Ordering::Relaxed),
            swap_out: SWAP_OUT_COUNT.load(Ordering::Relaxed),
            alloc_fail: SWAP_ALLOC_FAIL.load(Ordering::Relaxed),
            total_slots: total,
            free_slots: free,
        }
    }

    /// List registered areas.
    pub fn areas(&self) -> &[SwapArea] {
        &self.areas
    }

    /// Readahead slot count.
    pub fn readahead(&self) -> u32 {
        current_effective_swap_readahead(self.readahead_slots)
    }
}

#[inline(always)]
fn governor_adjusted_swap_readahead(slots: u32, latency_bias: &'static str) -> u32 {
    crate::kernel::virt_bias::adjust_budget_u32(slots.max(1), latency_bias)
}

#[inline(always)]
fn current_effective_swap_readahead(slots: u32) -> u32 {
    governor_adjusted_swap_readahead(slots, crate::kernel::virt_bias::current_latency_bias())
}

#[cfg(test)]
#[path = "swap/tests.rs"]
mod tests;
