use core::alloc::GlobalAlloc;

pub const PAGE_SIZE_4K: usize = 4096;
pub const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;
pub const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;

/// Page Allocator interface for physical memory.
/// Supports Huge Pages for TLB optimization.
pub trait PageAllocator {
    /// Allocate 2^order pages.
    /// Order 0 = 4KB, Order 9 = 2MB (Huge Page on x86).
    fn allocate_pages(&mut self, order: u8) -> Option<usize>;

    fn deallocate_pages(&mut self, addr: usize, order: u8);
}

/// Heap Allocator interface (extends GlobalAlloc)
pub trait HeapAllocator: GlobalAlloc {
    fn init(&self, start: usize, size: usize);
}
