use crate::interfaces::PageAllocator;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, Ordering};

/// Buddy System Allocator — Production Quality
///
/// Splits physical memory into power-of-2 blocks (order 0 = 4 KiB … order 11 = 8 MiB).
/// Features:
///   - O(log N) allocation and deallocation
///   - Coalescing (buddy merging) on free
///   - Per-order stats (allocations, deallocations, merges)
///   - Lockless fast-path: caller holds `crate::kernel::sync::IrqSafeMutex`
///
/// Usage invariant: all addresses must be within the initialised physical region.

pub const MAX_ORDER: usize = 12; // 2^11 × 4 KiB = 8 MiB max contiguous block
const PAGE_SHIFT: usize = 12; // 4 KiB pages

struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
}

/// Per-order statistics.
#[derive(Default, Clone, Copy, Debug)]
pub struct BuddyOrderStats {
    pub allocs: u64,
    pub deallocs: u64,
    pub merges: u64,
}

pub struct BuddyAllocator {
    free_lists: [Option<NonNull<FreeBlock>>; MAX_ORDER],
    start_addr: usize,
    end_addr: usize,
    total_pages: usize,
    total_alloc: AtomicU64,
    total_free: AtomicU64,
    oom_count: AtomicU64,
    /// Per-order stats (index = order).
    order_alloc: [AtomicU64; MAX_ORDER],
    order_merge: [AtomicU64; MAX_ORDER],
}

unsafe impl Send for BuddyAllocator {}

impl BuddyAllocator {
    pub const fn new() -> Self {
        // Can't use a loop in const context; initialise arrays manually.
        Self {
            free_lists: [None; MAX_ORDER],
            start_addr: 0,
            end_addr: 0,
            total_pages: 0,
            total_alloc: AtomicU64::new(0),
            total_free: AtomicU64::new(0),
            oom_count: AtomicU64::new(0),
            order_alloc: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            order_merge: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    /// Initialise the allocator with the given physical memory region.
    ///
    /// Populates the free lists by recursively splitting the region into
    /// the largest power-of-2 aligned blocks that fit.
    pub fn init(&mut self, start: usize, size: usize) {
        // Align start up to page boundary.
        let aligned_start = (start + (1 << PAGE_SHIFT) - 1) & !((1 << PAGE_SHIFT) - 1);
        let aligned_end = (start + size) & !((1 << PAGE_SHIFT) - 1);
        if aligned_end <= aligned_start {
            return;
        }

        self.start_addr = aligned_start;
        self.end_addr = aligned_end;
        self.total_pages = (aligned_end - aligned_start) >> PAGE_SHIFT;

        // Walk the region and add the largest aligned blocks we can.
        let mut addr = aligned_start;
        while addr < aligned_end {
            let remaining = aligned_end - addr;
            // Find the highest order block that (a) fits and (b) is naturally aligned.
            let mut order = MAX_ORDER - 1;
            loop {
                let block_size = 1usize << (order + PAGE_SHIFT);
                if block_size <= remaining && (addr % block_size) == 0 {
                    break;
                }
                if order == 0 {
                    break;
                }
                order -= 1;
            }
            let block_size = 1usize << (order + PAGE_SHIFT);
            if block_size > remaining {
                break;
            }
            unsafe {
                self.push_free(addr, order);
            }
            addr += block_size;
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    unsafe fn push_free(&mut self, addr: usize, order: usize) {
        // Safety: caller guarantees `addr` is allocator-managed storage for a free-list node.
        let mut node = unsafe { NonNull::new_unchecked(addr as *mut FreeBlock) };
        // Safety: `node` points to a valid intrusive free-list node owned by this allocator.
        unsafe { node.as_mut().next = self.free_lists[order] };
        self.free_lists[order] = Some(node);
    }

    /// Remove a specific address from the given order's free list.
    /// Returns true if the block was found and removed.
    unsafe fn remove_free(&mut self, addr: usize, order: usize) -> bool {
        let mut current = self.free_lists[order];
        let mut prev: Option<NonNull<FreeBlock>> = None;

        while let Some(node) = current {
            if node.as_ptr() as usize == addr {
                if let Some(mut p) = prev {
                    // Safety: both nodes are elements in the allocator-owned free list.
                    unsafe { p.as_mut().next = node.as_ref().next };
                } else {
                    // Safety: `node` is the current head node for this order.
                    self.free_lists[order] = unsafe { node.as_ref().next };
                }
                return true;
            }
            prev = current;
            // Safety: iterating across allocator-owned free-list nodes.
            current = unsafe { node.as_ref().next };
        }
        false
    }

    /// Compute the buddy address for a block at `addr` of `order`.
    #[inline]
    fn buddy_of(start: usize, addr: usize, order: usize) -> usize {
        let _block_size = 1usize << (order + PAGE_SHIFT);
        let frame = (addr - start) >> PAGE_SHIFT;
        let aligned = frame & !((1usize << (order)) - 1); // start of aligned pair
        let buddy_frame = aligned ^ (1 << order);
        start + (buddy_frame << PAGE_SHIFT)
    }

    // ── Public stats ─────────────────────────────────────────────────────────

    pub fn total_allocated_pages(&self) -> u64 {
        self.total_alloc.load(Ordering::Relaxed)
    }
    pub fn total_freed_pages(&self) -> u64 {
        self.total_free.load(Ordering::Relaxed)
    }
    pub fn oom_count(&self) -> u64 {
        self.oom_count.load(Ordering::Relaxed)
    }
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    pub fn free_pages(&self) -> usize {
        let alloc = self.total_alloc.load(Ordering::Relaxed) as usize;
        let freed = self.total_free.load(Ordering::Relaxed) as usize;
        self.total_pages.saturating_sub(alloc.saturating_sub(freed))
    }

    /// Snapshot order-level stats (allocs, merges) for each order 0..<MAX_ORDER.
    pub fn order_stats(&self) -> [BuddyOrderStats; MAX_ORDER] {
        let mut out = [BuddyOrderStats::default(); MAX_ORDER];
        for (i, s) in out.iter_mut().enumerate() {
            s.allocs = self.order_alloc[i].load(Ordering::Relaxed);
            s.merges = self.order_merge[i].load(Ordering::Relaxed);
        }
        out
    }
}

impl PageAllocator for BuddyAllocator {
    fn allocate_pages(&mut self, order: u8) -> Option<usize> {
        let order = order as usize;
        if order >= MAX_ORDER {
            return None;
        }

        // Find the smallest available order ≥ requested.
        for i in order..MAX_ORDER {
            if let Some(block_ptr) = self.free_lists[i] {
                // Remove from list.
                unsafe {
                    self.free_lists[i] = block_ptr.as_ref().next;
                }

                let addr = block_ptr.as_ptr() as usize;

                // Split higher-order block down to the requested order.
                let mut cur_order = i;
                while cur_order > order {
                    cur_order -= 1;
                    let buddy_addr = addr + (1usize << (cur_order + PAGE_SHIFT));
                    unsafe {
                        self.push_free(buddy_addr, cur_order);
                    }
                }

                self.total_alloc.fetch_add(1u64 << order, Ordering::Relaxed);
                self.order_alloc[order].fetch_add(1, Ordering::Relaxed);
                return Some(addr);
            }
        }

        self.oom_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn deallocate_pages(&mut self, addr: usize, order: u8) {
        let mut addr = addr;
        let mut order = order as usize;

        // Validate address.
        if addr < self.start_addr || addr >= self.end_addr {
            return;
        }

        self.total_free.fetch_add(1u64 << order, Ordering::Relaxed);

        // Attempt to coalesce with buddy.
        loop {
            if order >= MAX_ORDER - 1 {
                unsafe {
                    self.push_free(addr, order);
                }
                return;
            }

            let buddy_addr = Self::buddy_of(self.start_addr, addr, order);

            // Buddy must be within the managed region.
            if buddy_addr < self.start_addr || buddy_addr >= self.end_addr {
                unsafe {
                    self.push_free(addr, order);
                }
                return;
            }

            // Try to remove the buddy from the free list.
            if unsafe { self.remove_free(buddy_addr, order) } {
                // Merge: the lower address is the new merged block.
                addr = addr.min(buddy_addr);
                self.order_merge[order].fetch_add(1, Ordering::Relaxed);
                order += 1;
                // Continue coalescing at the higher order.
            } else {
                unsafe {
                    self.push_free(addr, order);
                }
                return;
            }
        }
    }
}
