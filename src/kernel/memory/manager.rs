use crate::interfaces::memory::{HeapAllocator, PageAllocator, PAGE_SIZE_2M, PAGE_SIZE_4K};
use alloc::collections::BTreeSet;

const FRAME_ORDER_4K: u8 = 0;
const HUGE_PAGE_TO_BASE_PAGE_RATIO: usize = PAGE_SIZE_2M / PAGE_SIZE_4K;
const FRAME_ORDER_2M_HUGE: u8 = HUGE_PAGE_TO_BASE_PAGE_RATIO.trailing_zeros() as u8;

#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub allocated_frames: usize,
    pub allocated_huge_frames: usize,
    pub peak_allocated_frames: usize,
    pub peak_allocated_huge_frames: usize,
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub invalid_deallocate_attempts: usize,
}

/// The Memory Manager holds the physical (PMM) and virtual (Heap) allocators.
/// It acts as the central interface for all memory operations for the kernel.
pub struct MemoryManager<P, H>
where
    P: PageAllocator,
    H: HeapAllocator,
{
    page_allocator: P,
    _heap_allocator: H,
    allocated_frames: BTreeSet<usize>,
    allocated_huge_frames: BTreeSet<usize>,
    peak_allocated_frames: usize,
    peak_allocated_huge_frames: usize,
    total_allocations: usize,
    total_deallocations: usize,
    invalid_deallocate_attempts: usize,
}

impl<P: PageAllocator, H: HeapAllocator> MemoryManager<P, H> {
    #[inline(always)]
    fn is_aligned(addr: usize, align: usize) -> bool {
        addr % align == 0
    }

    #[inline(always)]
    fn record_invalid_deallocate(&mut self, op: &'static str, addr: usize) {
        self.invalid_deallocate_attempts = self.invalid_deallocate_attempts.saturating_add(1);
        crate::klog_warn!("MemoryManager: {} addr={:#x}", op, addr);
    }

    #[inline(always)]
    fn record_allocator_anomaly(&mut self, op: &'static str, addr: usize) {
        self.invalid_deallocate_attempts = self.invalid_deallocate_attempts.saturating_add(1);
        crate::klog_warn!("MemoryManager: {} addr={:#x}", op, addr);
    }

    #[inline(always)]
    fn refresh_peaks(&mut self) {
        let frames = self.allocated_frames.len();
        let huge = self.allocated_huge_frames.len();
        if frames > self.peak_allocated_frames {
            self.peak_allocated_frames = frames;
        }
        if huge > self.peak_allocated_huge_frames {
            self.peak_allocated_huge_frames = huge;
        }
    }

    pub fn new(page_allocator: P, heap_allocator: H) -> Self {
        Self {
            page_allocator,
            _heap_allocator: heap_allocator,
            allocated_frames: BTreeSet::new(),
            allocated_huge_frames: BTreeSet::new(),
            peak_allocated_frames: 0,
            peak_allocated_huge_frames: 0,
            total_allocations: 0,
            total_deallocations: 0,
            invalid_deallocate_attempts: 0,
        }
    }

    /// Allocates a physical frame.
    /// Used by the kernel to build page tables.
    pub fn allocate_frame(&mut self) -> Option<usize> {
        let addr = self.page_allocator.allocate_pages(FRAME_ORDER_4K)?;
        if !self.allocated_frames.insert(addr) {
            self.record_allocator_anomaly("duplicate 4K allocation tracked", addr);
        }
        self.total_allocations = self.total_allocations.saturating_add(1);
        self.refresh_peaks();
        Some(addr)
    }

    /// Allocates a huge frame (2MB).
    pub fn allocate_huge_frame(&mut self) -> Option<usize> {
        let addr = self.page_allocator.allocate_pages(FRAME_ORDER_2M_HUGE)?;
        if !self.allocated_huge_frames.insert(addr) {
            self.record_allocator_anomaly("duplicate 2M allocation tracked", addr);
        }
        self.total_allocations = self.total_allocations.saturating_add(1);
        self.refresh_peaks();
        Some(addr)
    }

    pub fn deallocate_frame(&mut self, addr: usize) {
        if !Self::is_aligned(addr, PAGE_SIZE_4K) {
            self.record_invalid_deallocate("rejecting deallocate_frame for unaligned", addr);
            return;
        }
        if !self.allocated_frames.remove(&addr) {
            self.record_invalid_deallocate("skipping deallocate_frame for unknown", addr);
            return;
        }
        self.page_allocator.deallocate_pages(addr, FRAME_ORDER_4K);
        self.total_deallocations = self.total_deallocations.saturating_add(1);
    }

    pub fn deallocate_huge_frame(&mut self, addr: usize) {
        if !Self::is_aligned(addr, PAGE_SIZE_2M) {
            self.record_invalid_deallocate("rejecting deallocate_huge_frame for unaligned", addr);
            return;
        }
        if !self.allocated_huge_frames.remove(&addr) {
            self.record_invalid_deallocate("skipping deallocate_huge_frame for unknown", addr);
            return;
        }
        self.page_allocator
            .deallocate_pages(addr, FRAME_ORDER_2M_HUGE);
        self.total_deallocations = self.total_deallocations.saturating_add(1);
    }

    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            allocated_frames: self.allocated_frames.len(),
            allocated_huge_frames: self.allocated_huge_frames.len(),
            peak_allocated_frames: self.peak_allocated_frames,
            peak_allocated_huge_frames: self.peak_allocated_huge_frames,
            total_allocations: self.total_allocations,
            total_deallocations: self.total_deallocations,
            invalid_deallocate_attempts: self.invalid_deallocate_attempts,
        }
    }
}
