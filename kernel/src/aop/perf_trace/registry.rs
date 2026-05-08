use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use crate::kernel::sync::IrqSafeMutex;
use lazy_static::lazy_static;

#[derive(Debug, Default)]
pub struct PerfMetrics {
    pub call_count: AtomicU64,
    pub total_cycles: AtomicU64,
    pub max_cycles: AtomicU64,
    pub min_cycles: AtomicU64,
    pub threshold_exceeded: AtomicU64,
}

impl PerfMetrics {
    pub fn new() -> Self {
        Self {
            call_count: AtomicU64::new(0),
            total_cycles: AtomicU64::new(0),
            max_cycles: AtomicU64::new(0),
            min_cycles: AtomicU64::new(u64::MAX),
            threshold_exceeded: AtomicU64::new(0),
        }
    }

    pub fn record(&self, cycles: u64, threshold: u64) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_cycles.fetch_add(cycles, Ordering::Relaxed);
        
        let mut max = self.max_cycles.load(Ordering::Relaxed);
        while cycles > max {
            match self.max_cycles.compare_exchange_weak(max, cycles, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => max = actual,
            }
        }

        let mut min = self.min_cycles.load(Ordering::Relaxed);
        while cycles < min {
            match self.min_cycles.compare_exchange_weak(min, cycles, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => min = actual,
            }
        }

        if cycles > threshold {
            self.threshold_exceeded.fetch_add(1, Ordering::Relaxed);
        }
    }
}

lazy_static! {
    static ref METRICS_REGISTRY: IrqSafeMutex<BTreeMap<&'static str, PerfMetrics>> = 
        IrqSafeMutex::new(BTreeMap::new());
}

pub fn record_metric(name: &'static str, cycles: u64, threshold: u64) {
    let mut registry = METRICS_REGISTRY.lock();
    if !registry.contains_key(name) {
        registry.insert(name, PerfMetrics::new());
    }
    registry.get(name).unwrap().record(cycles, threshold);
}

pub fn dump_metrics() {
    let registry = METRICS_REGISTRY.lock();
    crate::klog_info!("--- AetherXOS Perf Metrics ---");
    for (name, metrics) in registry.iter() {
        let calls = metrics.call_count.load(Ordering::Relaxed);
        let total = metrics.total_cycles.load(Ordering::Relaxed);
        let max = metrics.max_cycles.load(Ordering::Relaxed);
        let min = metrics.min_cycles.load(Ordering::Relaxed);
        let exceeded = metrics.threshold_exceeded.load(Ordering::Relaxed);
        
        let avg = if calls > 0 { total / calls } else { 0 };
        
        crate::klog_info!(
            "{:<20} | calls: {:<6} | avg: {:<8} | max: {:<8} | min: {:<8} | exceeded: {:<4}",
            name, calls, avg, max, min, exceeded
        );
    }
}
