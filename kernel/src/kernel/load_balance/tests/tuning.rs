use super::*;

#[test_case]
fn virtualization_rebalance_tuning_prefers_latency_profile() {
    assert_eq!(
        virtualization_rebalance_tuning(
            "LatencyCritical",
            "latency-critical",
            "full",
            "low-latency"
        ),
        VirtualizationRebalanceTuning {
            threshold_divisor: 1,
            batch_multiplier: 1,
            prefer_local_skip_budget_divisor: 1,
        }
    );
    assert_eq!(
        virtualization_rebalance_tuning("Background", "background", "basic", "throughput"),
        VirtualizationRebalanceTuning {
            threshold_divisor: 1,
            batch_multiplier: 1,
            prefer_local_skip_budget_divisor: 1,
        }
    );
    assert_eq!(
        virtualization_rebalance_tuning("Balanced", "balanced", "full", "balanced"),
        VirtualizationRebalanceTuning {
            threshold_divisor: 1,
            batch_multiplier: 1,
            prefer_local_skip_budget_divisor: 1,
        }
    );
}

#[test_case]
fn rebalance_helper_arithmetic_tracks_tuning_inputs() {
    reset_rebalance_adaptive_state();
    let aggressive = VirtualizationRebalanceTuning {
        threshold_divisor: 2,
        batch_multiplier: 2,
        prefer_local_skip_budget_divisor: 2,
    };
    let baseline = VirtualizationRebalanceTuning {
        threshold_divisor: 1,
        batch_multiplier: 1,
        prefer_local_skip_budget_divisor: 1,
    };

    assert!(rebalance_threshold(aggressive) <= rebalance_threshold(baseline));
    assert!(rebalance_batch_size(aggressive) >= rebalance_batch_size(baseline));
    assert!(prefer_local_skip_budget(aggressive) <= prefer_local_skip_budget(baseline));
    reset_rebalance_adaptive_state();
}

#[test_case]
fn rebalance_helpers_adapt_to_tail_imbalance_history() {
    reset_rebalance_adaptive_state();
    let baseline = VirtualizationRebalanceTuning {
        threshold_divisor: 1,
        batch_multiplier: 1,
        prefer_local_skip_budget_divisor: 1,
    };

    let threshold_before = rebalance_threshold(baseline);
    let batch_before = rebalance_batch_size(baseline);
    let skip_budget_before = prefer_local_skip_budget(baseline);

    for _ in 0..32 {
        record_imbalance_histogram(24);
    }

    let threshold_after = rebalance_threshold(baseline);
    let batch_after = rebalance_batch_size(baseline);
    let skip_budget_after = prefer_local_skip_budget(baseline);

    assert!(threshold_after >= threshold_before);
    assert!(batch_after >= batch_before);
    assert!(skip_budget_after >= skip_budget_before);
    reset_rebalance_adaptive_state();
}

#[test_case]
fn rebalance_helpers_decay_stale_outlier_impact() {
    let baseline = VirtualizationRebalanceTuning {
        threshold_divisor: 1,
        batch_multiplier: 1,
        prefer_local_skip_budget_divisor: 1,
    };

    reset_rebalance_adaptive_state();
    for _ in 0..4 {
        record_imbalance_histogram(64);
    }
    for _ in 0..28 {
        record_imbalance_histogram(4);
    }
    let threshold_with_stale_outliers = rebalance_threshold(baseline);

    reset_rebalance_adaptive_state();
    for _ in 0..28 {
        record_imbalance_histogram(4);
    }
    for _ in 0..4 {
        record_imbalance_histogram(64);
    }
    let threshold_with_fresh_outliers = rebalance_threshold(baseline);

    assert!(threshold_with_fresh_outliers >= threshold_with_stale_outliers);
    reset_rebalance_adaptive_state();
}

#[test_case]
fn rebalance_decision_snapshot_records_reason_and_metrics() {
    reset_rebalance_adaptive_state();

    record_rebalance_decision(
        RebalanceDecisionReason::BelowThreshold,
        18,
        12,
        6,
        4,
        3,
        0,
    );

    let stats = stats_snapshot();
    assert_eq!(stats.last_reason, RebalanceDecisionReason::BelowThreshold);
    assert_eq!(stats.last_source_load, 18);
    assert_eq!(stats.last_target_load, 12);
    assert_eq!(stats.last_imbalance, 6);
    assert_eq!(stats.last_threshold, 4);
    assert_eq!(stats.last_batch, 3);
    assert_eq!(stats.last_moved, 0);

    reset_rebalance_adaptive_state();
}
