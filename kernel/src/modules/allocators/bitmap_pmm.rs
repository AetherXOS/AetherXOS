use crate::interfaces::{PageAllocator, PAGE_SIZE_4K};
use core::sync::atomic::{AtomicU64, Ordering};

/// Bitmap Physical Memory Manager (PMM).
///
/// Manages a contiguous physical region page-by-page.
/// Allocation is O(n) first-fit; deallocation is O(1).
/// Intended as a fallback / bootstrap allocator until the Buddy
/// system takes over for large physical regions.
///
/// # Safety
/// Uses a spin::Mutex-protected bitmap to be safe from concurrent
/// access without exposing `static mut`.

const MEMORY_POOL_SIZE_MB: usize = 128;
const TOTAL_PAGES: usize = (MEMORY_POOL_SIZE_MB * 1024 * 1024) / PAGE_SIZE_4K;
const BITMAP_SIZE: usize = TOTAL_PAGES / 8; // 1 bit per page

pub const PMM_TOTAL_PAGES: usize = TOTAL_PAGES;
pub const PMM_BITMAP_SIZE: usize = BITMAP_SIZE;

// ── Thread-safe bitmap ────────────────────────────────────────────────────────

/// Mutex-protected bitmap.  No more `static mut`.
static BITMAP: spin::Mutex<[u8; BITMAP_SIZE]> = spin::Mutex::new([0u8; BITMAP_SIZE]);

// ── Telemetry ─────────────────────────────────────────────────────────────────

static PMM_ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static PMM_ALLOC_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PMM_ALLOC_FAIL: AtomicU64 = AtomicU64::new(0);
static PMM_FREE_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct BitmapPmmStats {
    pub alloc_calls: u64,
    pub alloc_success: u64,
    pub alloc_fail: u64,
    pub free_calls: u64,
    pub free_pages: usize,
}

pub fn pmm_stats() -> BitmapPmmStats {
    BitmapPmmStats {
        alloc_calls: PMM_ALLOC_CALLS.load(Ordering::Relaxed),
        alloc_success: PMM_ALLOC_SUCCESS.load(Ordering::Relaxed),
        alloc_fail: PMM_ALLOC_FAIL.load(Ordering::Relaxed),
        free_calls: PMM_FREE_CALLS.load(Ordering::Relaxed),
        free_pages: get_free_pages(),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Count free pages without exposing raw bitmap access.
pub fn get_free_pages() -> usize {
    BITMAP.lock().iter().map(|b| b.count_zeros() as usize).sum()
}

// ── BitmapAllocator ───────────────────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
const DEFAULT_PMM_BASE: usize = 0x10_0000; // 1 MiB skip real-mode for x86_64

#[cfg(target_arch = "aarch64")]
const DEFAULT_PMM_BASE: usize = 0x4000_0000; // Typical starting DRAM base on AArch64 virt machines

pub struct BitmapAllocator {
    start_addr: usize,
}

impl BitmapAllocator {
    /// Create a new PMM.  `start_addr` skips reserved regions depending on arch.
    pub const fn new() -> Self {
        Self {
            start_addr: DEFAULT_PMM_BASE,
        }
    }

    /// Create with an explicit physical base.
    pub const fn with_base(start_addr: usize) -> Self {
        Self { start_addr }
    }

    /// Reserve a range of physical pages (e.g., kernel image, MMIO, ACPI tables).
    pub fn mark_used(&self, start_frame: usize, count: usize) {
        let mut bm = BITMAP.lock();
        for i in 0..count {
            let frame = start_frame + i;
            if frame < TOTAL_PAGES {
                bm[frame / 8] |= 1 << (frame % 8);
            }
        }
    }

    /// Release a range of physical pages (inverse of mark_used).
    pub fn mark_free(&self, start_frame: usize, count: usize) {
        let mut bm = BITMAP.lock();
        for i in 0..count {
            let frame = start_frame + i;
            if frame < TOTAL_PAGES {
                bm[frame / 8] &= !(1 << (frame % 8));
            }
        }
    }
}

impl PageAllocator for BitmapAllocator {
    /// Allocate 2^order naturally-aligned physical pages.
    fn allocate_pages(&mut self, order: u8) -> Option<usize> {
        PMM_ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);

        let pages_needed: usize = 1 << order;
        let mut bm = BITMAP.lock();
        let mut run_start = 0usize;
        let mut run_len = 0usize;

        for i in 0..TOTAL_PAGES {
            let used = (bm[i / 8] >> (i % 8)) & 1 == 1;
            if !used {
                if run_len == 0 {
                    // For power-of-2 aligned allocations, ensure the start is naturally aligned.
                    if pages_needed > 1 && (i % pages_needed) != 0 {
                        run_len = 0;
                        continue;
                    }
                    run_start = i;
                }
                run_len += 1;
                if run_len == pages_needed {
                    // Mark allocated.
                    for j in run_start..(run_start + pages_needed) {
                        bm[j / 8] |= 1 << (j % 8);
                    }
                    PMM_ALLOC_SUCCESS.fetch_add(1, Ordering::Relaxed);
                    return Some(self.start_addr + run_start * PAGE_SIZE_4K);
                }
            } else {
                run_len = 0;
            }
        }

        PMM_ALLOC_FAIL.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn deallocate_pages(&mut self, addr: usize, order: u8) {
        PMM_FREE_CALLS.fetch_add(1, Ordering::Relaxed);

        if addr < self.start_addr {
            return;
        }
        let start_frame = (addr - self.start_addr) / PAGE_SIZE_4K;
        let pages_count: usize = 1 << order;
        let mut bm = BITMAP.lock();

        for i in 0..pages_count {
            let frame = start_frame + i;
            if frame < TOTAL_PAGES {
                bm[frame / 8] &= !(1 << (frame % 8));
            }
        }
    }
}
