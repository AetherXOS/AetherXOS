use crate::interfaces::HeapAllocator;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

// A simple but thread-safe Bump Allocator.
// In a real OS, this would check against memory limits provided by the bootloader.

pub struct BumpAllocator {
    heap_start: AtomicUsize,
    heap_end: AtomicUsize,
    next: AtomicUsize,
    allocations: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: AtomicUsize::new(0),
            heap_end: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        // Lock-free reservation loop: retry CAS until reservation succeeds
        loop {
            // Load current pointer
            let current = self.next.load(Ordering::Acquire);

            // Calculate alignment
            let aligned = (current + align - 1) & !(align - 1);
            let new_next = aligned + size;

            if new_next > self.heap_end.load(Ordering::Relaxed) {
                return ptr::null_mut(); // OOM
            }

            // Attempt to update pointer
            if self
                .next
                .compare_exchange_weak(current, new_next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                self.allocations.fetch_add(1, Ordering::Relaxed);
                return aligned as *mut u8;
            }
            // If failed, loop again
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        self.allocations.fetch_sub(1, Ordering::Relaxed);
        // Bump allocator cannot reclaim memory
    }
}

impl HeapAllocator for BumpAllocator {
    fn init(&self, start: usize, size: usize) {
        self.heap_start.store(start, Ordering::SeqCst);
        self.heap_end.store(start + size, Ordering::SeqCst);
        self.next.store(start, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests;
