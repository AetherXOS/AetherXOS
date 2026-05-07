//! RTOS Jitter Monitoring & Deterministic Verification
//! 
//! Measures the deviation between the intended activation time of a task
//! and its actual execution start. This "jitter" is a critical metric for 
//! DO-178C and ISO 26262 certification.

use core::sync::atomic::{AtomicU64, Ordering};

static MAX_SCHED_JITTER_NS: AtomicU64 = AtomicU64::new(0);
static AVG_SCHED_JITTER_NS: AtomicU64 = AtomicU64::new(0);
static JITTER_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Record a scheduling event to track jitter.
/// `expected_ns`: The timestamp when the task SHOULD have run (e.g. deadline or period start).
/// `actual_ns`: The current timestamp (now).
pub fn record_scheduling_event(expected_ns: u64, actual_ns: u64) {
    if expected_ns == 0 || actual_ns <= expected_ns {
        return; 
    }
    
    let jitter = actual_ns - expected_ns;
    
    // Update Max Jitter (Lock-free max)
    let mut current_max = MAX_SCHED_JITTER_NS.load(Ordering::Relaxed);
    while jitter > current_max {
        match MAX_SCHED_JITTER_NS.compare_exchange_weak(
            current_max, 
            jitter, 
            Ordering::Relaxed, 
            Ordering::Relaxed
        ) {
            Ok(_) => break,
            Err(v) => current_max = v,
        }
    }
    
    // Update Average
    let count = JITTER_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let old_avg = AVG_SCHED_JITTER_NS.load(Ordering::Relaxed);
    let new_avg = (old_avg.saturating_mul(count - 1).saturating_add(jitter)) / count;
    AVG_SCHED_JITTER_NS.store(new_avg, Ordering::Relaxed);
}

/// Returns the maximum recorded jitter in nanoseconds.
pub fn get_max_jitter() -> u64 {
    MAX_SCHED_JITTER_NS.load(Ordering::Relaxed)
}

/// Returns the average recorded jitter in nanoseconds.
pub fn get_avg_jitter() -> u64 {
    AVG_SCHED_JITTER_NS.load(Ordering::Relaxed)
}

#[cfg(feature = "rtos_strict")]
pub fn check_jitter_violation(limit_ns: u64) {
    let max = MAX_SCHED_JITTER_NS.load(Ordering::Relaxed);
    if max > limit_ns {
        // Log violation for certification audit trail
        crate::klog_warn!("RTOS Jitter Violation: Max jitter reached {}ns (limit {}ns)", max, limit_ns);
    }
}
