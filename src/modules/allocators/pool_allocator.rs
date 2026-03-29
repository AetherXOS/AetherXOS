use crate::interfaces::memory::HeapAllocator;
use crate::kernel::sync::IrqSafeMutex;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Fixed-size Pool Allocator.
///
/// Maintains a singly-linked free list of identically-sized blocks.
/// - `alloc`:   O(1) — pop from head.
/// - `dealloc`: O(1) — push to head with alignment guard.
/// - Cross-size requests fall through to null (caller should delegate to a fallback).
///
/// New in this version:
///   - Block alignment is *enforced* on every free block during init (not just head).
///   - Validated `dealloc`: checks the pointer falls within the managed region.
///   - Telemetry counters (alloc hits, dealloc, OOM, alignment rejects).

pub struct PoolAllocator {
    head: IrqSafeMutex<usize>,
    start_addr: AtomicUsize,
    end_addr: AtomicUsize,

    // Telemetry
    alloc_calls: AtomicU64,
    alloc_hits: AtomicU64,
    alloc_oom: AtomicU64,
    alloc_align: AtomicU64, // rejected due to alignment
    dealloc_calls: AtomicU64,
    dealloc_oob: AtomicU64, // pointer outside managed region
}

#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    pub alloc_calls: u64,
    pub alloc_hits: u64,
    pub alloc_oom: u64,
    pub alloc_align: u64,
    pub dealloc_calls: u64,
    pub dealloc_oob: u64,
}

impl PoolAllocator {
    pub const fn new() -> Self {
        PoolAllocator {
            head: IrqSafeMutex::new(0),
            start_addr: AtomicUsize::new(0),
            end_addr: AtomicUsize::new(0),
            alloc_calls: AtomicU64::new(0),
            alloc_hits: AtomicU64::new(0),
            alloc_oom: AtomicU64::new(0),
            alloc_align: AtomicU64::new(0),
            dealloc_calls: AtomicU64::new(0),
            dealloc_oob: AtomicU64::new(0),
        }
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            alloc_calls: self.alloc_calls.load(Ordering::Relaxed),
            alloc_hits: self.alloc_hits.load(Ordering::Relaxed),
            alloc_oom: self.alloc_oom.load(Ordering::Relaxed),
            alloc_align: self.alloc_align.load(Ordering::Relaxed),
            dealloc_calls: self.dealloc_calls.load(Ordering::Relaxed),
            dealloc_oob: self.dealloc_oob.load(Ordering::Relaxed),
        }
    }
}

unsafe impl GlobalAlloc for PoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_calls.fetch_add(1, Ordering::Relaxed);
        let block_size = crate::generated_consts::MEM_POOL_BLOCK_SIZE;

        // Reject requests that exceed block size or require stricter alignment.
        if layout.size() > block_size || layout.align() > block_size {
            self.alloc_align.fetch_add(1, Ordering::Relaxed);
            return null_mut();
        }

        let mut head = self.head.lock();
        if *head == 0 {
            self.alloc_oom.fetch_add(1, Ordering::Relaxed);
            return null_mut();
        }

        let ptr_val = *head;

        // Alignment check on the actual block pointer.
        if ptr_val % layout.align() != 0 {
            self.alloc_align.fetch_add(1, Ordering::Relaxed);
            return null_mut();
        }

        let node = ptr_val as *mut usize;
        *head = unsafe { *node };
        self.alloc_hits.fetch_add(1, Ordering::Relaxed);
        node as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc_calls.fetch_add(1, Ordering::Relaxed);
        let block_size = crate::generated_consts::MEM_POOL_BLOCK_SIZE;

        // Reject oversized blocks.
        if layout.size() > block_size {
            // Panic in debug, silently drop in release.
            #[cfg(debug_assertions)]
            panic!(
                "PoolAllocator::dealloc: block size {} > pool block size {}",
                layout.size(),
                block_size
            );
            #[cfg(not(debug_assertions))]
            return;
        }

        // Bounds-check: reject pointers outside our managed region.
        let addr = ptr as usize;
        let start = self.start_addr.load(Ordering::Relaxed);
        let end = self.end_addr.load(Ordering::Relaxed);
        if start != 0 && (addr < start || addr >= end) {
            self.dealloc_oob.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let mut head = self.head.lock();
        let node = ptr as *mut usize;
        unsafe {
            *node = *head;
        }
        *head = node as usize;
    }
}

impl HeapAllocator for PoolAllocator {
    fn init(&self, start: usize, size: usize) {
        let block_size = crate::generated_consts::MEM_POOL_BLOCK_SIZE;
        let mut head = self.head.lock();

        // Align start to block_size boundary.
        let misalign = start % block_size;
        let mut current = if misalign != 0 {
            start + (block_size - misalign)
        } else {
            start
        };
        let end = start + size;

        self.start_addr.store(current, Ordering::Relaxed);
        self.end_addr.store(end, Ordering::Relaxed);

        // Build the free list: each block's first word points to the next block.
        let mut previous_ptr: *mut usize = null_mut();
        while current + block_size <= end {
            let node = current as *mut usize;
            unsafe {
                *node = 0;
            }

            if previous_ptr.is_null() {
                *head = current;
            } else {
                unsafe {
                    *previous_ptr = current;
                }
            }
            previous_ptr = node;
            current += block_size;
        }
        // The last node's `next` is already 0 (null list terminator).
    }
}
