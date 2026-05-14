use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use alloc::vec::Vec;

/// Aether-Uring: High-Performance Asynchronous I/O Engine.
/// Provides O(1) syscall overhead for massive I/O workloads.
pub struct IoUringEntry {
    pub opcode: u8,
    pub fd: u32,
    pub addr: u64,
    pub len: u32,
    pub offset: u64,
    pub user_data: u64,
}

#[repr(u8)]
pub enum IoOp {
    Read = 0,
    Write = 1,
    Fsync = 2,
    Accept = 3,
}

pub struct IoCompletionEntry {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
}

pub struct IoUring {
    pub sq_head: AtomicU32,
    pub sq_tail: AtomicU32,
    pub cq_head: AtomicU32,
    pub cq_tail: AtomicU32,
    // UnsafeCell provides interior mutability for the lock-free ring buffer slots
    pub entries: Vec<core::cell::UnsafeCell<IoUringEntry>>,
    pub cq_entries: Vec<core::cell::UnsafeCell<IoCompletionEntry>>,
}

// SAFETY: IoUring entries are only accessed when the caller holds appropriate
// index-based ownership (SQ tail slot is producer-owned until committed).
unsafe impl Send for IoUring {}
unsafe impl Sync for IoUring {}

impl IoUring {
    pub fn new(capacity: usize) -> Self {
        let mut cq_entries = Vec::with_capacity(capacity * 2);
        for _ in 0..capacity * 2 {
            cq_entries.push(IoCompletionEntry {
                user_data: 0, res: 0, flags: 0
            });
        }
        Self {
            sq_head: AtomicU32::new(0),
            sq_tail: AtomicU32::new(0),
            cq_head: AtomicU32::new(0),
            cq_tail: AtomicU32::new(0),
            entries: (0..capacity).map(|_| core::cell::UnsafeCell::new(IoUringEntry {
                opcode: 0, fd: 0, addr: 0, len: 0, offset: 0, user_data: 0
            })).collect(),
            cq_entries: (0..capacity * 2).map(|_| core::cell::UnsafeCell::new(IoCompletionEntry {
                user_data: 0, res: 0, flags: 0
            })).collect(),
        }
    }

    /// Submit entries (Wait-Free).
    pub fn submit_entry(&self, entry: IoUringEntry) -> Result<(), &'static str> {
        let tail = self.sq_tail.load(Ordering::Relaxed);
        let head = self.sq_head.load(Ordering::Acquire);
        
        if tail.wrapping_sub(head) >= self.entries.len() as u32 {
            return Err("SQ full");
        }

        unsafe {
            let slot = &self.entries[tail as usize % self.entries.len()];
            slot.get().write(entry);
        }

        self.sq_tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Process submissions (Kernel Side).
    pub fn kernel_process<F>(&self, mut handler: F) -> usize
    where
        F: FnMut(&IoUringEntry),
    {
        let mut processed = 0;
        let head = self.sq_head.load(Ordering::Relaxed);
        let tail = self.sq_tail.load(Ordering::Acquire);

        let mut current_head = head;
        while current_head != tail {
            // SAFETY: current_head is within [sq_head, sq_tail) range - producer-owned slot
            let entry = unsafe { &*self.entries[current_head as usize % self.entries.len()].get() };
            handler(entry);
            current_head = current_head.wrapping_add(1);
            processed += 1;
        }

        self.sq_head.store(current_head, Ordering::Release);
        processed
    }

    /// Push a completion entry (Wait-Free).
    pub fn push_completion(&self, user_data: u64, res: i32) {
        let tail = self.cq_tail.load(Ordering::Relaxed);
        let head = self.cq_head.load(Ordering::Acquire);

        if tail.wrapping_sub(head) >= self.cq_entries.len() as u32 {
            // CQ full - drop entry (caller should handle backpressure)
            return;
        }

        // SAFETY: tail is producer-owned until we store cq_tail
        unsafe {
            let slot = self.cq_entries[tail as usize % self.cq_entries.len()].get();
            (*slot).user_data = user_data;
            (*slot).res = res;
            (*slot).flags = 0;
        }

        self.cq_tail.store(tail.wrapping_add(1), Ordering::Release);
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_IO_URING_REGISTRY: Mutex<alloc::collections::BTreeMap<u32, Arc<IoUring>>> =
        Mutex::new(alloc::collections::BTreeMap::new());
}
