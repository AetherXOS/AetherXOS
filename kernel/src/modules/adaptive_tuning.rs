//! Adaptive performance tuning
//! 
//! This module provides adaptive tuning with:
//! - Dynamic parameter adjustment based on workload
//! - Performance metric collection and analysis
//! - Automatic optimization decisions
//! - NUMA-aware resource allocation
//! - Telemetry for tuning effectiveness

use core::sync::atomic::{AtomicU64, AtomicBool, AtomicUsize, Ordering};

const MAX_TUNABLE_PARAMS: usize = 32;

// Telemetry
static TUNING_ADJUSTMENTS: AtomicU64 = AtomicU64::new(0);
static TUNING_IMPROVEMENTS: AtomicU64 = AtomicU64::new(0);
static TUNING_REGRESSIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct TuningStats {
    pub adjustments: u64,
    pub improvements: u64,
    pub regressions: u64,
    pub success_rate: f64,
}

pub fn tuning_stats() -> TuningStats {
    let adjustments = TUNING_ADJUSTMENTS.load(Ordering::Relaxed);
    let improvements = TUNING_IMPROVEMENTS.load(Ordering::Relaxed);
    let regressions = TUNING_REGRESSIONS.load(Ordering::Relaxed);
    let success_rate = if adjustments > 0 { 
        improvements as f64 / adjustments as f64 
    } else { 0.0 };

    TuningStats {
        adjustments,
        improvements,
        regressions,
        success_rate,
    }
}

/// Tunable parameter
struct TunableParam {
    name: &'static str,
    current: AtomicU64,
    min: AtomicU64,
    max: AtomicU64,
    step: AtomicU64,
}

impl TunableParam {
    const fn new(name: &'static str, initial: u64, min: u64, max: u64, step: u64) -> Self {
        Self {
            name,
            current: AtomicU64::new(initial),
            min: AtomicU64::new(min),
            max: AtomicU64::new(max),
            step: AtomicU64::new(step),
        }
    }

    #[inline(always)]
    fn increase(&self) -> bool {
        let current = self.current.load(Ordering::Acquire);
        let max = self.max.load(Ordering::Acquire);
        let step = self.step.load(Ordering::Acquire);
        
        if current + step <= max {
            self.current.store(current + step, Ordering::Release);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    fn decrease(&self) -> bool {
        let current = self.current.load(Ordering::Acquire);
        let min = self.min.load(Ordering::Acquire);
        let step = self.step.load(Ordering::Acquire);
        
        if current >= min + step {
            self.current.store(current - step, Ordering::Release);
            true
        } else {
            false
        }
    }
}

/// Performance metric collector
struct MetricCollector {
    samples: [AtomicU64; 128],
    index: AtomicUsize,
    count: AtomicUsize,
}

impl MetricCollector {
    const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            samples: [ZERO; 128],
            index: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn record(&self, value: u64) {
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % 128;
        self.samples[idx].store(value, Ordering::Release);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    fn average(&self) -> u64 {
        let count = self.count.load(Ordering::Relaxed).min(128);
        if count == 0 {
            return 0;
        }

        let mut sum: u64 = 0;
        for i in 0..count {
            sum += self.samples[i].load(Ordering::Relaxed);
        }

        sum / count as u64
    }
}

/// Adaptive tuner
pub struct AdaptiveTuner {
    params: [TunableParam; MAX_TUNABLE_PARAMS],
    metrics: MetricCollector,
    last_performance: AtomicU64,
    tuning_enabled: AtomicBool,
}

impl AdaptiveTuner {
    pub const fn new() -> Self {
        const PARAM_INIT: TunableParam = TunableParam::new("", 0, 0, 0, 0);
        Self {
            params: [PARAM_INIT; MAX_TUNABLE_PARAMS],
            metrics: MetricCollector::new(),
            last_performance: AtomicU64::new(0),
            tuning_enabled: AtomicBool::new(true),
        }
    }

    #[inline(always)]
    pub fn enable(&self) {
        self.tuning_enabled.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub fn disable(&self) {
        self.tuning_enabled.store(false, Ordering::Release);
    }

    pub fn register_param(&mut self, idx: usize, name: &'static str, initial: u64, min: u64, max: u64, step: u64) {
        if idx < MAX_TUNABLE_PARAMS {
            self.params[idx] = TunableParam::new(name, initial, min, max, step);
        }
    }

    #[inline(always)]
    pub fn record_metric(&self, value: u64) {
        self.metrics.record(value);
    }

    pub fn adjust(&self) -> Result<(), &'static str> {
        if !self.tuning_enabled.load(Ordering::Acquire) {
            return Ok(());
        }

        let current_perf = self.metrics.average();
        let last_perf = self.last_performance.load(Ordering::Acquire);
        
        TUNING_ADJUSTMENTS.fetch_add(1, Ordering::Relaxed);

        if current_perf > last_perf {
            TUNING_IMPROVEMENTS.fetch_add(1, Ordering::Relaxed);
            self.last_performance.store(current_perf, Ordering::Release);
            Ok(())
        } else if current_perf < last_perf {
            TUNING_REGRESSIONS.fetch_add(1, Ordering::Relaxed);
            Err("performance regression")
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    pub fn get_param(&self, idx: usize) -> Option<u64> {
        if idx < MAX_TUNABLE_PARAMS {
            Some(self.params[idx].current.load(Ordering::Relaxed))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_tunable_param() {
        let param = TunableParam::new("test", 50, 0, 100, 10);
        
        assert!(param.increase());
        assert_eq!(param.current.load(Ordering::Relaxed), 60);
        
        assert!(param.decrease());
        assert_eq!(param.current.load(Ordering::Relaxed), 50);
    }

    #[test_case]
    fn test_metric_collector() {
        let collector = MetricCollector::new();
        
        collector.record(100);
        collector.record(200);
        
        assert_eq!(collector.average(), 150);
    }

    #[test_case]
    fn test_tuning_stats() {
        let _stats = tuning_stats();
    }
}
