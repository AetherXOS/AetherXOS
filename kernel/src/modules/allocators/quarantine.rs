//! Kernel Heap Quarantine Mode
//!
//! Provides a delayed-free quarantine queue for use-after-free (UAF) detection.
//! When a pointer is freed, instead of immediately returning it to the pool,
//! it is placed in the quarantine queue. The memory is optionally poisoned (overwritten
//! with a magic pattern like 0xDFDFDFDF). Once the quarantine queue reaches its capacity,
//! the oldest pointers are finally handed back to the real allocator.

use core::sync::atomic::{AtomicUsize, AtomicBool, AtomicU8, Ordering};

const QUARANTINE_CAPACITY: usize = 4096; // Increased for better protection
static POISON_BYTE: AtomicU8 = AtomicU8::new(0xDF);
static QUARANTINE_ENABLED: AtomicBool = AtomicBool::new(true);

/// Lock-free storage for (ptr, size).
struct QuarantineEntry {
    ptr: AtomicUsize,
    size: AtomicUsize,
}

static QUARANTINE_STORAGE: [QuarantineEntry; QUARANTINE_CAPACITY] = unsafe {
    core::mem::transmute([(0usize, 0usize); QUARANTINE_CAPACITY])
};

static QUARANTINE_HEAD: AtomicUsize = AtomicUsize::new(0);
static QUARANTINE_TAIL: AtomicUsize = AtomicUsize::new(0);

/// Adds a pointer to the quarantine queue (Lock-Free).
pub fn quarantine_free<F: FnOnce(*mut u8, usize)>(ptr: *mut u8, size: usize, real_free: F) {
    if !QUARANTINE_ENABLED.load(Ordering::Relaxed) || ptr.is_null() {
        if !ptr.is_null() { real_free(ptr, size); }
        return;
    }

    // 1. Poison the memory (UAF Detection)
    unsafe {
        core::ptr::write_bytes(ptr, POISON_BYTE.load(Ordering::Relaxed), size);
    }

    // 2. Try to enqueue in the lock-free ring buffer
    let head = QUARANTINE_HEAD.fetch_add(1, Ordering::SeqCst) % QUARANTINE_CAPACITY;
    let entry = &QUARANTINE_STORAGE[head];

    // If there was an old pointer at this slot, we must free it (Eviction)
    let old_ptr = entry.ptr.swap(ptr as usize, Ordering::SeqCst);
    let old_size = entry.size.swap(size, Ordering::SeqCst);

    if old_ptr != 0 {
        // Safe eviction: this pointer has been in quarantine for QUARANTINE_CAPACITY frees
        real_free(old_ptr as *mut u8, old_size);
    }
}

/// Flushes the entire quarantine queue (Lock-Free).
pub fn flush_quarantine<F: FnMut(*mut u8, usize)>(mut real_free: F) {
    for entry in &QUARANTINE_STORAGE {
        let ptr = entry.ptr.swap(0, Ordering::SeqCst);
        let size = entry.size.swap(0, Ordering::SeqCst);
        if ptr != 0 {
            real_free(ptr as *mut u8, size);
        }
    }
}

pub fn set_quarantine_enabled(enabled: bool) {
    QUARANTINE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_poison_byte(byte: u8) {
    POISON_BYTE.store(byte, Ordering::Relaxed);
}
