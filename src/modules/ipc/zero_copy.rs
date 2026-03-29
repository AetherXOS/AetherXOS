use crate::interfaces::IpcChannel;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const ZERO_COPY_MAX_LEN: usize = 64 * 1024;

static ZERO_COPY_SET_BUFFER_CALLS: AtomicU64 = AtomicU64::new(0);
static ZERO_COPY_SEND_CALLS: AtomicU64 = AtomicU64::new(0);
static ZERO_COPY_SEND_DROPS_OVERSIZE: AtomicU64 = AtomicU64::new(0);
static ZERO_COPY_RECEIVE_CALLS: AtomicU64 = AtomicU64::new(0);
static ZERO_COPY_RECEIVE_HITS: AtomicU64 = AtomicU64::new(0);
static ZERO_COPY_RECEIVE_SMALL_BUFFER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct ZeroCopyStats {
    pub set_buffer_calls: u64,
    pub send_calls: u64,
    pub send_drops_oversize: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
    pub receive_small_buffer: u64,
}

pub fn stats() -> ZeroCopyStats {
    ZeroCopyStats {
        set_buffer_calls: ZERO_COPY_SET_BUFFER_CALLS.load(Ordering::Relaxed),
        send_calls: ZERO_COPY_SEND_CALLS.load(Ordering::Relaxed),
        send_drops_oversize: ZERO_COPY_SEND_DROPS_OVERSIZE.load(Ordering::Relaxed),
        receive_calls: ZERO_COPY_RECEIVE_CALLS.load(Ordering::Relaxed),
        receive_hits: ZERO_COPY_RECEIVE_HITS.load(Ordering::Relaxed),
        receive_small_buffer: ZERO_COPY_RECEIVE_SMALL_BUFFER.load(Ordering::Relaxed),
    }
}

use crate::interfaces::task::TaskId;

/// Zero-Copy IPC Implementation.
/// Instead of copying data, it transfers ownership of memory pages or maps shared memory.
/// This implementation uses a shared buffer address as a demonstration.
pub struct ZeroCopy {
    shared_buffer_ptr: AtomicUsize,
    buffer_len: AtomicUsize,
    owner: AtomicUsize,
}

impl ZeroCopy {
    pub const fn new() -> Self {
        Self {
            shared_buffer_ptr: AtomicUsize::new(0),
            buffer_len: AtomicUsize::new(0),
            owner: AtomicUsize::new(0),
        }
    }

    /// Set the shared buffer for IPC.
    /// In a real exokernel, this would involve remapping page table entries.
    pub fn set_buffer(&self, ptr: usize, len: usize) {
        ZERO_COPY_SET_BUFFER_CALLS.fetch_add(1, Ordering::Relaxed);

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        self.shared_buffer_ptr.store(ptr, Ordering::Release);
        self.buffer_len.store(len, Ordering::Release);
        self.owner.store(tid, Ordering::Release);
    }
}

impl IpcChannel for ZeroCopy {
    /// "Send" a message by updating the shared pointer.
    /// In true Zero-Copy, we just ensure the receiver can see the sender's memory.
    fn send(&self, msg: &[u8]) {
        ZERO_COPY_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        if msg.len() > ZERO_COPY_MAX_LEN {
            ZERO_COPY_SEND_DROPS_OVERSIZE.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        let ptr = msg.as_ptr() as usize;
        let len = msg.len();
        self.shared_buffer_ptr.store(ptr, Ordering::Release);
        self.buffer_len.store(len, Ordering::Release);
        self.owner.store(tid, Ordering::Release);
    }

    /// "Receive" a message by reading from the shared pointer into the buffer.
    fn receive(&self, buffer: &mut [u8]) -> Option<usize> {
        ZERO_COPY_RECEIVE_CALLS.fetch_add(1, Ordering::Relaxed);
        let ptr = self.shared_buffer_ptr.load(Ordering::Acquire);
        let len = self.buffer_len.load(Ordering::Acquire);
        let owner = self.owner.load(Ordering::Acquire);

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        if ptr == 0 || len == 0 {
            None
        } else if owner == tid {
            // Self-receive is allowed but usually not what IPC is for.
            // In a real OS, we'd check if `tid` has permission to read `owner`'s memory.
            None
        } else if len > buffer.len() {
            ZERO_COPY_RECEIVE_SMALL_BUFFER.fetch_add(1, Ordering::Relaxed);
            None
        } else {
            // Safety: In a real OS, ensure memory is mapped and sender hasn't unmapped it.
            // Here we assume existence for the sake of the demonstration.
            unsafe {
                core::ptr::copy_nonoverlapping(ptr as *const u8, buffer.as_mut_ptr(), len);
            }
            ZERO_COPY_RECEIVE_HITS.fetch_add(1, Ordering::Relaxed);
            Some(len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn zero_copy_rejects_oversize_send() {
        let zc = ZeroCopy::new();
        let payload = [0u8; ZERO_COPY_MAX_LEN + 1];
        zc.send(&payload);
        let mut out = [0u8; 16];
        assert_eq!(zc.receive(&mut out), None);
    }

    #[test_case]
    fn zero_copy_rejects_small_receive_buffer() {
        let zc = ZeroCopy::new();
        zc.set_buffer(0x1000, 32);
        let mut out = [0u8; 8];
        assert_eq!(zc.receive(&mut out), None);
    }
}
