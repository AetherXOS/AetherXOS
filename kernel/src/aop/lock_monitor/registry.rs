use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use crate::kernel::sync::IrqSafeMutex;
use lazy_static::lazy_static;

#[derive(Debug, Default)]
pub struct LockStats {
    pub wait_count: AtomicU64,
    pub total_wait_cycles: AtomicU64,
    pub total_hold_cycles: AtomicU64,
    pub max_hold_cycles: AtomicU64,
    pub contention_events: AtomicU64,
}

impl LockStats {
    pub fn new() -> Self {
        Self {
            wait_count: AtomicU64::new(0),
            total_wait_cycles: AtomicU64::new(0),
            total_hold_cycles: AtomicU64::new(0),
            max_hold_cycles: AtomicU64::new(0),
            contention_events: AtomicU64::new(0),
        }
    }

    pub fn record_wait(&self, cycles: u64) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        self.total_wait_cycles.fetch_add(cycles, Ordering::Relaxed);
        if cycles > 0 {
            self.contention_events.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_hold(&self, cycles: u64) {
        self.total_hold_cycles.fetch_add(cycles, Ordering::Relaxed);
        
        let mut max = self.max_hold_cycles.load(Ordering::Relaxed);
        while cycles > max {
            match self.max_hold_cycles.compare_exchange_weak(max, cycles, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => max = actual,
            }
        }
    }
}

lazy_static! {
    static ref LOCK_REGISTRY: IrqSafeMutex<BTreeMap<&'static str, LockStats>> = 
        IrqSafeMutex::new(BTreeMap::new());
}

pub fn record_lock_stats(name: &'static str, wait_cycles: u64, hold_cycles: u64) {
    let mut registry = LOCK_REGISTRY.lock();
    if !registry.contains_key(name) {
        registry.insert(name, LockStats::new());
    }
    let stats = registry.get(name).unwrap();
    stats.record_wait(wait_cycles);
    stats.record_hold(hold_cycles);
}

pub fn dump_lock_stats() {
    let registry = LOCK_REGISTRY.lock();
    crate::klog_info!("--- AetherXOS Lock Monitor Stats ---");
    for (name, stats) in registry.iter() {
        let waits = stats.wait_count.load(Ordering::Relaxed);
        let total_wait = stats.total_wait_cycles.load(Ordering::Relaxed);
        let total_hold = stats.total_hold_cycles.load(Ordering::Relaxed);
        let max_hold = stats.max_hold_cycles.load(Ordering::Relaxed);
        let contention = stats.contention_events.load(Ordering::Relaxed);
        
        let avg_wait = if waits > 0 { total_wait / waits } else { 0 };
        let avg_hold = if waits > 0 { total_hold / waits } else { 0 };
        
        crate::klog_info!(
            "{:<20} | waits: {:<6} | avg_wait: {:<8} | avg_hold: {:<8} | max_hold: {:<8} | contentions: {:<4}",
            name, waits, avg_wait, avg_hold, max_hold, contention
        );
    }
}
