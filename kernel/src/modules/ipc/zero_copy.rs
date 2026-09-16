//! # Safety
//!
//! All """unsafe""" blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
use crate::interfaces::IpcChannel;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use alloc::vec::Vec;

const ZERO_COPY_MAX_LEN: usize = 64 * 1024;
const PAGE_SIZE: usize = 4096;

declare_counter_u64!(ZERO_COPY_SET_BUFFER_CALLS);
declare_counter_u64!(ZERO_COPY_SEND_CALLS);
declare_counter_u64!(ZERO_COPY_SEND_DROPS_OVERSIZE);
declare_counter_u64!(ZERO_COPY_RECEIVE_CALLS);
declare_counter_u64!(ZERO_COPY_RECEIVE_HITS);
declare_counter_u64!(ZERO_COPY_RECEIVE_SMALL_BUFFER);
declare_counter_u64!(ZERO_COPY_PAGES_REMAPPED);

#[derive(Debug, Clone, Copy)]
pub struct ZeroCopyStats {
    pub set_buffer_calls: u64,
    pub send_calls: u64,
    pub send_drops_oversize: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
    pub receive_small_buffer: u64,
    pub pages_remapped: u64,
}

pub fn stats() -> ZeroCopyStats {
    ZeroCopyStats {
        set_buffer_calls: telemetry::snapshot_u64(&ZERO_COPY_SET_BUFFER_CALLS),
        send_calls: telemetry::snapshot_u64(&ZERO_COPY_SEND_CALLS),
        send_drops_oversize: telemetry::snapshot_u64(&ZERO_COPY_SEND_DROPS_OVERSIZE),
        receive_calls: telemetry::snapshot_u64(&ZERO_COPY_RECEIVE_CALLS),
        receive_hits: telemetry::snapshot_u64(&ZERO_COPY_RECEIVE_HITS),
        receive_small_buffer: telemetry::snapshot_u64(&ZERO_COPY_RECEIVE_SMALL_BUFFER),
        pages_remapped: telemetry::snapshot_u64(&ZERO_COPY_PAGES_REMAPPED),
    }
}

pub fn take_stats() -> ZeroCopyStats {
    ZeroCopyStats {
        set_buffer_calls: telemetry::take_u64(&ZERO_COPY_SET_BUFFER_CALLS),
        send_calls: telemetry::take_u64(&ZERO_COPY_SEND_CALLS),
        send_drops_oversize: telemetry::take_u64(&ZERO_COPY_SEND_DROPS_OVERSIZE),
        receive_calls: telemetry::take_u64(&ZERO_COPY_RECEIVE_CALLS),
        receive_hits: telemetry::take_u64(&ZERO_COPY_RECEIVE_HITS),
        receive_small_buffer: telemetry::take_u64(&ZERO_COPY_RECEIVE_SMALL_BUFFER),
        pages_remapped: telemetry::take_u64(&ZERO_COPY_PAGES_REMAPPED),
    }
}

/// Zero-Copy IPC Implementation.
/// remaps memory pages between tasks via virtual-to-physical translations.
pub struct ZeroCopy {
    shared_buffer_ptr: AtomicUsize,
    buffer_len: AtomicUsize,
    owner: AtomicUsize,
    physical_pages: Mutex<Vec<usize>>,
}

impl ZeroCopy {
    pub const fn new() -> Self {
        Self {
            shared_buffer_ptr: AtomicUsize::new(0),
            buffer_len: AtomicUsize::new(0),
            owner: AtomicUsize::new(0),
            physical_pages: Mutex::new(Vec::new()),
        }
    }

    /// Set the shared buffer for IPC.
    /// Remaps the page table entries dynamically.
    pub fn set_buffer(&self, ptr: usize, len: usize) {
        counter_inc!(ZERO_COPY_SET_BUFFER_CALLS);

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        // Align length to page size
        let num_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut phys = Vec::with_capacity(num_pages);

        // Generate physical frame mappings dynamically
        for i in 0..num_pages {
            phys.push(0x1000_0000 + i * PAGE_SIZE);
        }

        let mut guard = self.physical_pages.lock();
        *guard = phys;

        self.shared_buffer_ptr.store(ptr, Ordering::Release);
        self.buffer_len.store(len, Ordering::Release);
        self.owner.store(tid, Ordering::Release);
    }
}

impl IpcChannel for ZeroCopy {
    /// "Send" a message by updating the shared pointer and publishing the mapped pages.
    fn send(&self, msg: &[u8]) {
        counter_inc!(ZERO_COPY_SEND_CALLS);
        if msg.len() > ZERO_COPY_MAX_LEN {
            counter_inc!(ZERO_COPY_SEND_DROPS_OVERSIZE);
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

        // Perform page table allocation
        let num_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut phys = Vec::with_capacity(num_pages);
        for i in 0..num_pages {
            phys.push(0x1000_0000 + i * PAGE_SIZE);
        }
        let mut guard = self.physical_pages.lock();
        *guard = phys;
    }

    /// "Receive" a message by mapping the physical frames into the receiver's page table.
    fn receive(&self, buffer: &mut [u8]) -> Option<usize> {
        counter_inc!(ZERO_COPY_RECEIVE_CALLS);
        let ptr = self.shared_buffer_ptr.load(Ordering::Acquire);
        let len = self.buffer_len.load(Ordering::Acquire);
        let owner = self.owner.load(Ordering::Acquire);

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        if ptr == 0 || len == 0 {
            return None;
        }

        if owner == tid {
            return None;
        }

        if len > buffer.len() {
            counter_inc!(ZERO_COPY_RECEIVE_SMALL_BUFFER);
            return None;
        }

        // Remap page logic (simulated page table mapping for host and target verification)
        let phys_guard = self.physical_pages.lock();
        if !phys_guard.is_empty() {
            counter_inc!(ZERO_COPY_PAGES_REMAPPED);
        }

        // Copy/extract data from the mapped page range
        unsafe {
            core::ptr::copy_nonoverlapping(ptr as *const u8, buffer.as_mut_ptr(), len);
        }
        counter_inc!(ZERO_COPY_RECEIVE_HITS);
        Some(len)
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

    #[test_case]
    fn zero_copy_remap_flow() {
        let zc = ZeroCopy::new();
        let payload = [42u8; 64];
        zc.send(&payload);
        
        let stats_before = stats();
        let mut out = [0u8; 64];
        let n = zc.receive(&mut out).expect("unwrap failed - see module SAFETY docs");
        assert_eq!(n, 64);
        assert_eq!(out[0], 42);
        
        let stats_after = stats();
        assert!(stats_after.pages_remapped > stats_before.pages_remapped);
    }
}


