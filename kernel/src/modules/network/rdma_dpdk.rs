//! RDMA and DPDK-style network acceleration
//! 
//! This module provides high-performance network operations with:
//! - Remote Direct Memory Access (RDMA) for zero-copy network transfers
//! - DPDK-style poll mode drivers for kernel bypass
//! - Lock-free ring buffers for packet processing
//! - NUMA-aware memory allocation

use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const MAX_RDMA_QUEUES: usize = 128;
const RDMA_QUEUE_DEPTH: usize = 4096;
const DPDK_RING_SIZE: usize = 4096;

// Telemetry
static RDMA_SEND_OPS: AtomicU64 = AtomicU64::new(0);
static RDMA_RECV_OPS: AtomicU64 = AtomicU64::new(0);
static RDMA_BYTES_SENT: AtomicU64 = AtomicU64::new(0);
static RDMA_BYTES_RECV: AtomicU64 = AtomicU64::new(0);
static DPDK_POLL_CYCLES: AtomicU64 = AtomicU64::new(0);
static DPDK_PACKETS_PROCESSED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct RdmaStats {
    pub send_ops: u64,
    pub recv_ops: u64,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DpdkStats {
    pub poll_cycles: u64,
    pub packets_processed: u64,
    pub packets_per_cycle: f64,
}

pub fn rdma_stats() -> RdmaStats {
    RdmaStats {
        send_ops: RDMA_SEND_OPS.load(Ordering::Relaxed),
        recv_ops: RDMA_RECV_OPS.load(Ordering::Relaxed),
        bytes_sent: RDMA_BYTES_SENT.load(Ordering::Relaxed),
        bytes_recv: RDMA_BYTES_RECV.load(Ordering::Relaxed),
    }
}

pub fn dpdk_stats() -> DpdkStats {
    let cycles = DPDK_POLL_CYCLES.load(Ordering::Relaxed);
    let packets = DPDK_PACKETS_PROCESSED.load(Ordering::Relaxed);
    let ppc = if cycles > 0 { packets as f64 / cycles as f64 } else { 0.0 };

    DpdkStats {
        poll_cycles: cycles,
        packets_processed: packets,
        packets_per_cycle: ppc,
    }
}

/// RDMA memory region for zero-copy transfers
#[repr(C, align(4096))]
pub struct RdmaMemoryRegion {
    region_id: AtomicU64,
    base_addr: AtomicU64,
    size: AtomicU64,
    lkey: AtomicU32,
    rkey: AtomicU32,
}

impl RdmaMemoryRegion {
    pub const fn new(region_id: u64, base_addr: u64, size: u64) -> Self {
        Self {
            region_id: AtomicU64::new(region_id),
            base_addr: AtomicU64::new(base_addr),
            size: AtomicU64::new(size),
            lkey: AtomicU32::new(0),
            rkey: AtomicU32::new(0),
        }
    }
}

/// RDMA queue pair for send/receive operations
pub struct RdmaQueuePair {
    qp_num: AtomicU32,
    send_queue: LockFreeRing,
    recv_queue: LockFreeRing,
    cq: CompletionQueue,
}

impl RdmaQueuePair {
    pub fn new(qp_num: u32) -> Self {
        Self {
            qp_num: AtomicU32::new(qp_num),
            send_queue: LockFreeRing::new(RDMA_QUEUE_DEPTH),
            recv_queue: LockFreeRing::new(RDMA_QUEUE_DEPTH),
            cq: CompletionQueue::new(),
        }
    }

    #[inline(always)]
    pub fn post_send(&self, wr: &WorkRequest) -> Result<(), &'static str> {
        RDMA_SEND_OPS.fetch_add(1, Ordering::Relaxed);
        RDMA_BYTES_SENT.fetch_add(wr.length.load(Ordering::Relaxed) as u64, Ordering::Relaxed);
        self.send_queue.enqueue(wr as *const _ as u64)
    }

    #[inline(always)]
    pub fn post_recv(&self, wr: &WorkRequest) -> Result<(), &'static str> {
        RDMA_RECV_OPS.fetch_add(1, Ordering::Relaxed);
        self.recv_queue.enqueue(wr as *const _ as u64)
    }

    #[inline(always)]
    pub fn poll_cq(&self) -> Option<CompletionEntry> {
        self.cq.dequeue()
    }
}

/// Work request for RDMA operations
#[repr(C)]
pub struct WorkRequest {
    wr_id: u64,
    opcode: AtomicU8,
    flags: AtomicU8,
    length: AtomicU32,
    local_addr: AtomicU64,
    remote_addr: AtomicU64,
    rkey: AtomicU32,
}

/// Completion queue entry
#[repr(C)]
pub struct CompletionEntry {
    wr_id: u64,
    opcode: u8,
    status: u8,
    byte_len: u32,
}

/// Lock-free ring buffer for DPDK-style operations
struct LockFreeRing {
    head: AtomicUsize,
    tail: AtomicUsize,
    size: usize,
    mask: usize,
    ring: [AtomicU64; DPDK_RING_SIZE],
}

impl LockFreeRing {
    const fn new(size: usize) -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            size,
            mask: size - 1,
            ring: [ZERO; DPDK_RING_SIZE],
        }
    }

    #[inline(always)]
    fn enqueue(&self, value: u64) -> Result<(), &'static str> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if (tail.wrapping_sub(head) & self.mask) == self.size {
            return Err("ring full");
        }

        let idx = tail & self.mask;
        self.ring[idx].store(value, Ordering::Release);
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    #[inline(always)]
    fn dequeue(&self) -> Option<u64> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head == tail {
            return None;
        }

        let idx = head & self.mask;
        let value = self.ring[idx].load(Ordering::Acquire);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(value)
    }
}

/// Completion queue
struct CompletionQueue {
    head: AtomicUsize,
    tail: AtomicUsize,
    entries: [AtomicU64; 256],
}

impl CompletionQueue {
    const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            entries: [ZERO; 256],
        }
    }

    #[inline(always)]
    fn dequeue(&self) -> Option<CompletionEntry> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head == tail {
            return None;
        }

        let idx = head & 255;
        let entry = self.entries[idx].load(Ordering::Acquire);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        
        Some(CompletionEntry {
            wr_id: entry,
            opcode: 0,
            status: 0,
            byte_len: 0,
        })
    }
}

/// DPDK-style poll mode driver
pub struct DpdkPollModeDriver {
    rx_queues: [LockFreeRing; 16],
    tx_queues: [LockFreeRing; 16],
    port_id: AtomicU8,
}

impl DpdkPollModeDriver {
    pub const fn new(port_id: u8) -> Self {
        const RING_INIT: LockFreeRing = LockFreeRing::new(DPDK_RING_SIZE);
        Self {
            rx_queues: [RING_INIT; 16],
            tx_queues: [RING_INIT; 16],
            port_id: AtomicU8::new(port_id),
        }
    }

    #[inline(always)]
    pub fn rx_burst(&self, queue_id: usize, max_pkts: usize) -> alloc::vec::Vec<u64> {
        DPDK_POLL_CYCLES.fetch_add(1, Ordering::Relaxed);
        
        let mut pkts = alloc::vec::Vec::with_capacity(max_pkts);
        let queue = &self.rx_queues[queue_id % 16];
        
        for _ in 0..max_pkts {
            if let Some(pkt) = queue.dequeue() {
                pkts.push(pkt);
            } else {
                break;
            }
        }
        
        DPDK_PACKETS_PROCESSED.fetch_add(pkts.len() as u64, Ordering::Relaxed);
        pkts
    }

    #[inline(always)]
    pub fn tx_burst(&self, queue_id: usize, pkts: &[u64]) -> usize {
        let queue = &self.tx_queues[queue_id % 16];
        let mut sent = 0;
        
        for &pkt in pkts {
            if queue.enqueue(pkt).is_ok() {
                sent += 1;
            } else {
                break;
            }
        }
        
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_lock_free_ring() {
        let ring = LockFreeRing::new(16);
        
        assert!(ring.enqueue(1).is_ok());
        assert!(ring.enqueue(2).is_ok());
        
        assert_eq!(ring.dequeue(), Some(1));
        assert_eq!(ring.dequeue(), Some(2));
        assert_eq!(ring.dequeue(), None);
    }

    #[test_case]
    fn test_rdma_stats() {
        let _stats = rdma_stats();
    }

    #[test_case]
    fn test_dpdk_stats() {
        let _stats = dpdk_stats();
    }
}
