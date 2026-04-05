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
}
