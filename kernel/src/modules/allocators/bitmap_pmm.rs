use crate::interfaces::{PageAllocator, PAGE_SIZE_4K};
use crate::hal::common::mmio;
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

    /// Initialise (or re-initialise) the physical base used by this instance.
    pub fn init(&mut self, start_addr: usize, _size: usize) {
        self.start_addr = start_addr;
        // Reserve kernel physical footprint if linker symbols are available.
        // The linker provides `_kernel_start` and `_kernel_end` as virtual
        // addresses (higher-half). Convert to physical using HHDM mapping
        // and mark those frames as used so PMM won't hand them out.
        unsafe extern "C" {
            static _kernel_start: u8;
            static _kernel_end: u8;
        }

        // SAFETY: reading addresses of linker-provided symbols is safe.
        let kstart_v = unsafe { &_kernel_start as *const u8 as usize };
        let kend_v = unsafe { &_kernel_end as *const u8 as usize };

        if kend_v > kstart_v {
            if let Some(kstart_phys) = mmio::virt_to_phys(kstart_v) {
                if let Some(kend_phys) = mmio::virt_to_phys(kend_v.saturating_sub(1)) {
                    let phys_start = kstart_phys as usize;
                    let phys_end_excl = (kend_phys as usize).saturating_add(1);

                    if phys_end_excl > self.start_addr {
                        let start_frame = if phys_start <= self.start_addr {
                            0usize
                        } else {
                            (phys_start - self.start_addr) / PAGE_SIZE_4K
                        };
                        let end_frame_excl = (phys_end_excl.saturating_sub(self.start_addr) + PAGE_SIZE_4K - 1) / PAGE_SIZE_4K;
                        if end_frame_excl > start_frame {
                            let count = end_frame_excl - start_frame;
                            // Mark kernel pages as used in the bitmap so allocator won't reuse them.
                            self.mark_used(start_frame, count);
                        }
                    }
                }
            }
        }
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
