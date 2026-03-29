use super::{
    BackpressurePolicy, NetworkAlertReport, NetworkAlertThresholds, LATENCY_TICKS_BUCKET_0,
    LATENCY_TICKS_BUCKET_1, LATENCY_TICKS_BUCKET_2_3_MAX, LATENCY_TICKS_BUCKET_4_7_MAX,
    LATENCY_TICKS_BUCKET_GE8, PERCENTILE_P50, PERCENTILE_P95, PERCENTILE_P99,
    PERCENTILE_ROUNDING_BIAS, PERCENTILE_SCALE,
};
use core::sync::atomic::{AtomicU64, Ordering};

pub(super) fn policy_to_u64(policy: BackpressurePolicy) -> u64 {
    match policy {
        BackpressurePolicy::Drop => super::BACKPRESSURE_POLICY_DROP_RAW,
        BackpressurePolicy::Defer => super::BACKPRESSURE_POLICY_DEFER_RAW,
        BackpressurePolicy::ForcePoll => super::BACKPRESSURE_POLICY_FORCE_POLL_RAW,
    }
}

pub(super) fn policy_from_u64(raw: u64) -> BackpressurePolicy {
    match raw {
        super::BACKPRESSURE_POLICY_DEFER_RAW => BackpressurePolicy::Defer,
        super::BACKPRESSURE_POLICY_FORCE_POLL_RAW => BackpressurePolicy::ForcePoll,
        _ => BackpressurePolicy::Drop,
    }
}

pub(super) fn compute_network_alert_report(
    health_score: u64,
    total_drops: u64,
    peak_queue: u64,
    thresholds: NetworkAlertThresholds,
) -> NetworkAlertReport {
    let health_breach = health_score < thresholds.min_health_score;
    let drops_breach = total_drops > thresholds.max_drops;
    let queue_breach = peak_queue > thresholds.max_queue_high_water;
    let breach_count = (health_breach as u8) + (drops_breach as u8) + (queue_breach as u8);

    NetworkAlertReport {
        health_breach,
        drops_breach,
        queue_breach,
        breach_count,
    }
}

pub(super) fn percentile_from_latency_buckets(
    total: u64,
    b0: u64,
    b1: u64,
    b2_3: u64,
    b4_7: u64,
    bge8: u64,
    percentile: u64,
) -> u64 {
    if total == 0 {
        return LATENCY_TICKS_BUCKET_0;
    }
    let rank = total
        .saturating_mul(percentile)
        .saturating_add(PERCENTILE_ROUNDING_BIAS)
        / PERCENTILE_SCALE;

    let mut cumulative = b0;
    if cumulative >= rank {
        return LATENCY_TICKS_BUCKET_0;
    }
    cumulative = cumulative.saturating_add(b1);
    if cumulative >= rank {
        return LATENCY_TICKS_BUCKET_1;
    }
    cumulative = cumulative.saturating_add(b2_3);
    if cumulative >= rank {
        return LATENCY_TICKS_BUCKET_2_3_MAX;
    }
    cumulative = cumulative.saturating_add(b4_7);
    if cumulative >= rank {
        return LATENCY_TICKS_BUCKET_4_7_MAX;
    }
    let _ = bge8;
    LATENCY_TICKS_BUCKET_GE8
}

pub(super) fn latency_percentiles(
    total: u64,
    b0: u64,
    b1: u64,
    b2_3: u64,
    b4_7: u64,
    bge8: u64,
) -> (u64, u64, u64) {
    (
        percentile_from_latency_buckets(total, b0, b1, b2_3, b4_7, bge8, PERCENTILE_P50),
        percentile_from_latency_buckets(total, b0, b1, b2_3, b4_7, bge8, PERCENTILE_P95),
        percentile_from_latency_buckets(total, b0, b1, b2_3, b4_7, bge8, PERCENTILE_P99),
    )
}

pub(super) fn update_high_water(mark: &AtomicU64, depth: usize) {
    let depth = depth as u64;
    let mut current = mark.load(Ordering::Relaxed);
    while depth > current {
        match mark.compare_exchange_weak(current, depth, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

pub(super) fn record_latency_bucket(
    delta_ticks: u64,
    b0: &AtomicU64,
    b1: &AtomicU64,
    b2_3: &AtomicU64,
    b4_7: &AtomicU64,
    bge8: &AtomicU64,
) {
    match delta_ticks {
        0 => {
            b0.fetch_add(1, Ordering::Relaxed);
        }
        1 => {
            b1.fetch_add(1, Ordering::Relaxed);
        }
        2..=3 => {
            b2_3.fetch_add(1, Ordering::Relaxed);
        }
        4..=7 => {
            b4_7.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            bge8.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(super) fn reset_counter(counter: &AtomicU64, value: u64) {
    counter.store(value, Ordering::Relaxed);
}

pub(super) fn reset_counters(counters: &[&AtomicU64], value: u64) {
    for counter in counters {
        reset_counter(counter, value);
    }
}

pub(super) fn reset_latency_buckets(
    b0: &AtomicU64,
    b1: &AtomicU64,
    b2_3: &AtomicU64,
    b4_7: &AtomicU64,
    bge8: &AtomicU64,
) {
    reset_counter(b0, 0);
    reset_counter(b1, 0);
    reset_counter(b2_3, 0);
    reset_counter(b4_7, 0);
    reset_counter(bge8, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicU64;

    #[test_case]
    fn policy_roundtrip_defaults_unknown_to_drop() {
        assert_eq!(
            policy_from_u64(policy_to_u64(BackpressurePolicy::Drop)),
            BackpressurePolicy::Drop
        );
        assert_eq!(
            policy_from_u64(policy_to_u64(BackpressurePolicy::Defer)),
            BackpressurePolicy::Defer
        );
        assert_eq!(
            policy_from_u64(policy_to_u64(BackpressurePolicy::ForcePoll)),
            BackpressurePolicy::ForcePoll
        );
        assert_eq!(policy_from_u64(u64::MAX), BackpressurePolicy::Drop);
    }

    #[test_case]
    fn latency_percentiles_progress_monotonically_across_buckets() {
        let (p50, p95, p99) = latency_percentiles(10, 2, 2, 2, 2, 2);
        assert!(p50 <= p95);
        assert!(p95 <= p99);
        assert_eq!(latency_percentiles(0, 0, 0, 0, 0, 0), (0, 0, 0));
    }

    #[test_case]
    fn compute_network_alert_report_counts_breaches() {
        let report = compute_network_alert_report(
            4,
            9,
            12,
            NetworkAlertThresholds {
                min_health_score: 5,
                max_drops: 8,
                max_queue_high_water: 10,
            },
        );
        assert!(report.health_breach);
        assert!(report.drops_breach);
        assert!(report.queue_breach);
        assert_eq!(report.breach_count, 3);
    }

    #[test_case]
    fn high_water_and_latency_bucket_helpers_are_stable() {
        let mark = AtomicU64::new(4);
        update_high_water(&mark, 2);
        assert_eq!(mark.load(Ordering::Relaxed), 4);
        update_high_water(&mark, 9);
        assert_eq!(mark.load(Ordering::Relaxed), 9);

        let b0 = AtomicU64::new(0);
        let b1 = AtomicU64::new(0);
        let b2_3 = AtomicU64::new(0);
        let b4_7 = AtomicU64::new(0);
        let bge8 = AtomicU64::new(0);
        record_latency_bucket(0, &b0, &b1, &b2_3, &b4_7, &bge8);
        record_latency_bucket(1, &b0, &b1, &b2_3, &b4_7, &bge8);
        record_latency_bucket(3, &b0, &b1, &b2_3, &b4_7, &bge8);
        record_latency_bucket(6, &b0, &b1, &b2_3, &b4_7, &bge8);
        record_latency_bucket(9, &b0, &b1, &b2_3, &b4_7, &bge8);
        assert_eq!(b0.load(Ordering::Relaxed), 1);
        assert_eq!(b1.load(Ordering::Relaxed), 1);
        assert_eq!(b2_3.load(Ordering::Relaxed), 1);
        assert_eq!(b4_7.load(Ordering::Relaxed), 1);
        assert_eq!(bge8.load(Ordering::Relaxed), 1);
    }

    #[test_case]
    fn reset_counter_overwrites_previous_value() {
        let mark = AtomicU64::new(77);
        reset_counter(&mark, 3);
        assert_eq!(mark.load(Ordering::Relaxed), 3);
    }

    #[test_case]
    fn reset_counters_updates_whole_group() {
        let a = AtomicU64::new(7);
        let b = AtomicU64::new(9);
        reset_counters(&[&a, &b], 4);
        assert_eq!(a.load(Ordering::Relaxed), 4);
        assert_eq!(b.load(Ordering::Relaxed), 4);
    }

    #[test_case]
    fn reset_latency_buckets_clears_all_series() {
        let b0 = AtomicU64::new(1);
        let b1 = AtomicU64::new(2);
        let b2_3 = AtomicU64::new(3);
        let b4_7 = AtomicU64::new(4);
        let bge8 = AtomicU64::new(5);
        reset_latency_buckets(&b0, &b1, &b2_3, &b4_7, &bge8);
        assert_eq!(b0.load(Ordering::Relaxed), 0);
        assert_eq!(b1.load(Ordering::Relaxed), 0);
        assert_eq!(b2_3.load(Ordering::Relaxed), 0);
        assert_eq!(b4_7.load(Ordering::Relaxed), 0);
        assert_eq!(bge8.load(Ordering::Relaxed), 0);
    }
}
