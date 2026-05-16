//! Aether-URing: High-performance, lock-free, asynchronous syscall interface.
//!
//! Inspired by Linux io_uring, this system uses shared-memory ring buffers 
//! (Submission Queue and Completion Queue) to eliminate context switch overhead.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::interfaces::KernelResult;
use crate::kernel::syscalls::SyscallFrame;

/// Maximum number of entries in the ring buffer.
pub const URING_ENTRIES: usize = 4096;

/// A Submission Queue Entry (SQE)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubmissionEntry {
    pub opcode: u32,
    pub fd: i32,
    pub addr: u64,
    pub len: u32,
    pub flags: u32,
    pub user_data: u64,
    pub args: [u64; 3], // Additional arguments (a3, a4, a5)
}

/// A Completion Queue Entry (CQE)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CompletionEntry {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
}

/// The shared memory structure for Aether-URing.
/// This is mapped into both kernel and userspace.
#[repr(C)]
pub struct AsyncRing {
    // --- Submission Queue (SQ) ---
    pub sq_head: AtomicU32,
    pub sq_tail: AtomicU32,
    pub sq_mask: u32,
    pub sq_entries: [SubmissionEntry; URING_ENTRIES],

    // --- Completion Queue (CQ) ---
    pub cq_head: AtomicU32,
    pub cq_tail: AtomicU32,
    pub cq_mask: u32,
    pub cq_entries: [CompletionEntry; URING_ENTRIES],
    
    pub flags: AtomicU32,
}

impl AsyncRing {
    /// Create a new ring in kernel memory.
    pub fn new() -> Self {
        Self {
            sq_head: AtomicU32::new(0),
            sq_tail: AtomicU32::new(0),
            sq_mask: (URING_ENTRIES - 1) as u32,
            sq_entries: [SubmissionEntry { 
                opcode: 0, fd: 0, addr: 0, len: 0, flags: 0, user_data: 0, args: [0; 3] 
            }; URING_ENTRIES],
            
            cq_head: AtomicU32::new(0),
            cq_tail: AtomicU32::new(0),
            cq_mask: (URING_ENTRIES - 1) as u32,
            cq_entries: [CompletionEntry { user_data: 0, res: 0, flags: 0 }; URING_ENTRIES],
            
            flags: AtomicU32::new(0),
        }
    }

    /// Process pending submission entries.
    /// This is called by the kernel (e.g., in a dedicated worker or end of syscall).
    pub fn poll(&self) -> u32 {
        let head = self.sq_head.load(Ordering::Acquire);
        let tail = self.sq_tail.load(Ordering::Acquire);
        
        let mut processed = 0;
        let mut current_head = head;

        while current_head != tail {
            let index = (current_head & self.sq_mask) as usize;
            let sqe = &self.sq_entries[index];
            
            // Dispatch the async syscall
            let result = self.dispatch_sqe(sqe);
            
            // Push to Completion Queue
            self.push_completion(sqe.user_data, result);
            
            current_head = current_head.wrapping_add(1);
            processed += 1;
        }

        self.sq_head.store(current_head, Ordering::Release);
        processed
    }

    fn dispatch_sqe(&self, sqe: &SubmissionEntry) -> i32 {
        // Construct a pseudo-frame for the dispatcher
        let mut frame = SyscallFrame {
            rax: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rcx: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            rip: 0,
            rflags: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rsp: 0,
        };

        // Map submission entry fields to syscall argument registers (rdi, rsi, rdx, r10, r8, r9)
        frame.rdi = sqe.fd as u64;
        frame.rsi = sqe.addr as u64;
        frame.rdx = sqe.len as u64;
        frame.r10 = sqe.args[0] as u64;
        frame.r8 = sqe.args[1] as u64;
        frame.r9 = sqe.args[2] as u64;

        // Dispatch via O(1) table (Ring 3 Service)
        let result = super::SyscallDispFrame::dispatch_structured(sqe.opcode as usize, &mut frame);
        result as i32
    }

    fn push_completion(&self, user_data: u64, res: i32) {
        let head = self.cq_head.load(Ordering::Acquire);
        let tail = self.cq_tail.load(Ordering::Acquire);
        
        // Check for overflow (simplified)
        if tail.wrapping_sub(head) >= URING_ENTRIES as u32 {
            return; 
        }

        let index = (tail & self.cq_mask) as usize;
        let cqe = &mut unsafe { 
            // In a real production system, this would be a shared pointer to CQ entries
            // but for this implementation we use the direct array.
            let ptr = self as *const Self as *mut Self;
            &mut (*ptr).cq_entries[index]
        };

        cqe.user_data = user_data;
        cqe.res = res;
        cqe.flags = 0;

        self.cq_tail.store(tail.wrapping_add(1), Ordering::Release);
    }
}
