use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const CORE_FRAME_QUEUE_LIMIT: usize = 1024;

static RX_ENQUEUE_CALLS: AtomicU64 = AtomicU64::new(0);
static RX_DEQUEUE_CALLS: AtomicU64 = AtomicU64::new(0);
static RX_DROPS: AtomicU64 = AtomicU64::new(0);
static TX_ENQUEUE_CALLS: AtomicU64 = AtomicU64::new(0);
static TX_DEQUEUE_CALLS: AtomicU64 = AtomicU64::new(0);
static TX_DROPS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref RX_QUEUE: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
    static ref TX_QUEUE: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
}

#[derive(Debug, Clone, Copy)]
pub struct NetCoreStats {
    pub rx_enqueue_calls: u64,
    pub rx_dequeue_calls: u64,
    pub rx_drops: u64,
    pub tx_enqueue_calls: u64,
    pub tx_dequeue_calls: u64,
    pub tx_drops: u64,
    pub rx_depth: usize,
    pub tx_depth: usize,
    pub queue_limit: usize,
}

pub fn submit_rx_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    RX_ENQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut queue = RX_QUEUE.lock();
    if queue.len() >= CORE_FRAME_QUEUE_LIMIT {
        RX_DROPS.fetch_add(1, Ordering::Relaxed);
        return Err("core rx queue full");
    }
    queue.push_back(frame);
    Ok(())
}

pub fn take_rx_frame() -> Option<Vec<u8>> {
    RX_DEQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
    RX_QUEUE.lock().pop_front()
}

pub fn drain_rx_frames(max_frames: usize) -> Vec<Vec<u8>> {
    if max_frames == 0 {
        return Vec::new();
    }

    let mut queue = RX_QUEUE.lock();
    let take = core::cmp::min(max_frames, queue.len());
    let mut frames = Vec::with_capacity(take);
    for _ in 0..take {
        if let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }
    }
    RX_DEQUEUE_CALLS.fetch_add(frames.len() as u64, Ordering::Relaxed);
    frames
}

pub fn submit_rx_frames(frames: Vec<Vec<u8>>) -> usize {
    let mut queue = RX_QUEUE.lock();
    let mut accepted = 0usize;
    for frame in frames {
        RX_ENQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
        if queue.len() >= CORE_FRAME_QUEUE_LIMIT {
            RX_DROPS.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        queue.push_back(frame);
        accepted += 1;
    }
    accepted
}

pub fn submit_tx_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    TX_ENQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut queue = TX_QUEUE.lock();
    if queue.len() >= CORE_FRAME_QUEUE_LIMIT {
        TX_DROPS.fetch_add(1, Ordering::Relaxed);
        return Err("core tx queue full");
    }
    queue.push_back(frame);
    Ok(())
}

pub fn take_tx_frame() -> Option<Vec<u8>> {
    TX_DEQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
    TX_QUEUE.lock().pop_front()
}

pub fn drain_tx_frames(max_frames: usize) -> Vec<Vec<u8>> {
    if max_frames == 0 {
        return Vec::new();
    }

    let mut queue = TX_QUEUE.lock();
    let take = core::cmp::min(max_frames, queue.len());
    let mut frames = Vec::with_capacity(take);
    for _ in 0..take {
        if let Some(frame) = queue.pop_front() {
            frames.push(frame);
        }
    }
    TX_DEQUEUE_CALLS.fetch_add(frames.len() as u64, Ordering::Relaxed);
    frames
}

pub fn submit_tx_frames(frames: Vec<Vec<u8>>) -> usize {
    let mut queue = TX_QUEUE.lock();
    let mut accepted = 0usize;
    for frame in frames {
        TX_ENQUEUE_CALLS.fetch_add(1, Ordering::Relaxed);
        if queue.len() >= CORE_FRAME_QUEUE_LIMIT {
            TX_DROPS.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        queue.push_back(frame);
        accepted += 1;
    }
    accepted
}

pub fn queue_limit() -> usize {
    CORE_FRAME_QUEUE_LIMIT
}

pub fn stats() -> NetCoreStats {
    NetCoreStats {
        rx_enqueue_calls: RX_ENQUEUE_CALLS.load(Ordering::Relaxed),
        rx_dequeue_calls: RX_DEQUEUE_CALLS.load(Ordering::Relaxed),
        rx_drops: RX_DROPS.load(Ordering::Relaxed),
        tx_enqueue_calls: TX_ENQUEUE_CALLS.load(Ordering::Relaxed),
        tx_dequeue_calls: TX_DEQUEUE_CALLS.load(Ordering::Relaxed),
        tx_drops: TX_DROPS.load(Ordering::Relaxed),
        rx_depth: RX_QUEUE.lock().len(),
        tx_depth: TX_QUEUE.lock().len(),
        queue_limit: CORE_FRAME_QUEUE_LIMIT,
    }
}
