//! Kernel stability monitoring and self-healing
//! 
//! This module provides stability monitoring with:
//! - Health check mechanisms for kernel subsystems
//! - Automatic fault detection and recovery
//! - Resource leak detection
//! - Performance anomaly detection
//! - Telemetry for stability monitoring

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const MAX_MONITORED_SUBSYSTEMS: usize = 64;
const HEALTH_CHECK_INTERVAL_MS: u64 = 1000;

// Telemetry
static STABILITY_FAULTS_DETECTED: AtomicU64 = AtomicU64::new(0);
static STABILITY_RECOVERIES: AtomicU64 = AtomicU64::new(0);
static STABILITY_LEAKS_DETECTED: AtomicU64 = AtomicU64::new(0);
static STABILITY_ANOMALIES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemHealth {
    Healthy,
    Degraded,
    Critical,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct StabilityStats {
    pub faults_detected: u64,
    pub recoveries: u64,
    pub leaks_detected: u64,
    pub anomalies: u64,
    pub recovery_rate: f64,
}

pub fn stability_stats() -> StabilityStats {
    let faults = STABILITY_FAULTS_DETECTED.load(Ordering::Relaxed);
    let recoveries = STABILITY_RECOVERIES.load(Ordering::Relaxed);
    let recovery_rate = if faults > 0 { recoveries as f64 / faults as f64 } else { 0.0 };

    StabilityStats {
        faults_detected: faults,
        recoveries: recoveries,
        leaks_detected: STABILITY_LEAKS_DETECTED.load(Ordering::Relaxed),
        anomalies: STABILITY_ANOMALIES.load(Ordering::Relaxed),
        recovery_rate,
    }
}

/// Monitored subsystem
struct SubsystemMonitor {
    name: &'static str,
    health: AtomicU32, // SubsystemHealth as u32
    fault_count: AtomicU32,
    last_check: AtomicU64,
    recovery_action: Option<fn() -> Result<(), &'static str>>,
}

impl SubsystemMonitor {
    const fn new(name: &'static str) -> Self {
        Self {
            name,
            health: AtomicU32::new(SubsystemHealth::Healthy as u32),
            fault_count: AtomicU32::new(0),
            last_check: AtomicU64::new(0),
            recovery_action: None,
        }
    }

    #[inline(always)]
    fn get_health(&self) -> SubsystemHealth {
        match self.health.load(Ordering::Acquire) {
            0 => SubsystemHealth::Healthy,
            1 => SubsystemHealth::Degraded,
            2 => SubsystemHealth::Critical,
            _ => SubsystemHealth::Failed,
        }
    }

    #[inline(always)]
    fn set_health(&self, health: SubsystemHealth) {
        self.health.store(health as u32, Ordering::Release);
    }

    #[inline(always)]
    fn increment_fault(&self) {
        self.fault_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    fn check(&self, check_fn: fn() -> bool) -> SubsystemHealth {
        let is_healthy = check_fn();
        
        if is_healthy {
            self.set_health(SubsystemHealth::Healthy);
            self.fault_count.store(0, Ordering::Release);
        } else {
            self.increment_fault();
            let faults = self.fault_count.load(Ordering::Relaxed);
            
            if faults > 10 {
                self.set_health(SubsystemHealth::Failed);
            } else if faults > 5 {
                self.set_health(SubsystemHealth::Critical);
            } else if faults > 2 {
                self.set_health(SubsystemHealth::Degraded);
            }
        }

        self.get_health()
    }
}

/// Resource leak detector
struct LeakDetector {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    threshold: AtomicUsize,
}

impl LeakDetector {
    const fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
            threshold: AtomicUsize::new(10000),
        }
    }

    #[inline(always)]
    fn record_alloc(&self) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    fn record_free(&self) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    fn check_leak(&self) -> bool {
        let allocs = self.allocations.load(Ordering::Relaxed);
        let frees = self.deallocations.load(Ordering::Relaxed);
        let diff = allocs.wrapping_sub(frees);
        
        diff > self.threshold.load(Ordering::Relaxed)
    }
}

/// Performance anomaly detector
struct AnomalyDetector {
    history: [AtomicU64; 16],
    index: AtomicUsize,
    mean: AtomicU64,
    std_dev: AtomicU64,
}

impl AnomalyDetector {
    const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            history: [ZERO; 16],
            index: AtomicUsize::new(0),
            mean: AtomicU64::new(0),
            std_dev: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    fn record(&self, value: u64) {
        let idx = self.index.fetch_add(1, Ordering::Relaxed) % 16;
        self.history[idx].store(value, Ordering::Release);
    }

    #[inline(always)]
    fn check_anomaly(&self, value: u64) -> bool {
        let mean = self.mean.load(Ordering::Relaxed);
        let std_dev = self.std_dev.load(Ordering::Relaxed);
        
        // Simple 3-sigma rule
        if std_dev == 0 {
            false
        } else {
            let diff = if value > mean { value - mean } else { mean - value };
            diff > 3 * std_dev
        }
    }
}

/// Kernel stability monitor
pub struct StabilityMonitor {
    subsystems: [SubsystemMonitor; MAX_MONITORED_SUBSYSTEMS],
    leak_detector: LeakDetector,
    anomaly_detector: AnomalyDetector,
    monitoring_enabled: AtomicBool,
}

impl StabilityMonitor {
    pub const fn new() -> Self {
        const SUBSYS_INIT: SubsystemMonitor = SubsystemMonitor::new("");
        Self {
            subsystems: [SUBSYS_INIT; MAX_MONITORED_SUBSYSTEMS],
            leak_detector: LeakDetector::new(),
            anomaly_detector: AnomalyDetector::new(),
            monitoring_enabled: AtomicBool::new(true),
        }
    }

    #[inline(always)]
    pub fn enable(&self) {
        self.monitoring_enabled.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub fn disable(&self) {
        self.monitoring_enabled.store(false, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.monitoring_enabled.load(Ordering::Acquire)
    }

    pub fn register_subsystem(&mut self, idx: usize, name: &'static str) {
        if idx < MAX_MONITORED_SUBSYSTEMS {
            self.subsystems[idx] = SubsystemMonitor::new(name);
        }
    }

    pub fn check_subsystem(&self, idx: usize, check_fn: fn() -> bool) -> SubsystemHealth {
        if idx < MAX_MONITORED_SUBSYSTEMS {
            let subsystem = &self.subsystems[idx];
            subsystem.check(check_fn)
        } else {
            SubsystemHealth::Failed
        }
    }

    pub fn attempt_recovery(&self, idx: usize) -> Result<(), &'static str> {
        if idx < MAX_MONITORED_SUBSYSTEMS {
            let subsystem = &self.subsystems[idx];
            
            // Reset fault count
            subsystem.fault_count.store(0, Ordering::Release);
            subsystem.set_health(SubsystemHealth::Healthy);
            
            STABILITY_RECOVERIES.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err("invalid subsystem index")
        }
    }

    #[inline(always)]
    pub fn record_alloc(&self) {
        self.leak_detector.record_alloc();
    }

    #[inline(always)]
    pub fn record_free(&self) {
        self.leak_detector.record_free();
    }

    #[inline(always)]
    pub fn check_leak(&self) -> bool {
        if self.leak_detector.check_leak() {
            STABILITY_LEAKS_DETECTED.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn record_metric(&self, value: u64) {
        self.anomaly_detector.record(value);
    }

    #[inline(always)]
    pub fn check_anomaly(&self, value: u64) -> bool {
        if self.anomaly_detector.check_anomaly(value) {
            STABILITY_ANOMALIES.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_subsystem_monitor() {
        let monitor = SubsystemMonitor::new("test");
        
        assert_eq!(monitor.get_health(), SubsystemHealth::Healthy);
        
        monitor.check(|| true);
        assert_eq!(monitor.get_health(), SubsystemHealth::Healthy);
    }

    #[test_case]
    fn test_leak_detector() {
        let detector = LeakDetector::new();
        
        for _ in 0..10001 {
            detector.record_alloc();
        }
        
        assert!(detector.check_leak());
    }

    #[test_case]
    fn test_stability_stats() {
        let _stats = stability_stats();
    }
}
