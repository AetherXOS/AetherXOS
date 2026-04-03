use crate::interfaces::HeapAllocator;
use crate::kernel::sync::IrqSafeMutex;
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::Heap;

// Wrapper around the linked_list_allocator crate which provides a decent
// first-fit/best-fit allocator suitable for kernel usage.
// We use IrqSafeMutex to prevent deadlocks when allocating from ISRs.

pub struct LinkedListAllocator {
    heap: IrqSafeMutex<Heap>,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            heap: IrqSafeMutex::new(Heap::empty()),
        }
    }
}

#[cfg(test)]
mod tests;

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.heap
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.heap
                .lock()
                .deallocate(core::ptr::NonNull::new_unchecked(ptr), layout);
        }
    }
}

impl HeapAllocator for LinkedListAllocator {
    fn init(&self, start: usize, size: usize) {
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw("[EARLY SERIAL] linked list heap init begin\n");
        unsafe {
            self.heap.lock().init(start as *mut u8, size);
        }
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw("[EARLY SERIAL] linked list heap init returned\n");
    }
}
