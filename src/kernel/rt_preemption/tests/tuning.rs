use super::*;

#[test_case]
fn virtualization_scheduler_tuning_prefers_latency_and_background_profiles() {
    assert_eq!(
        virtualization_scheduler_tuning(
            "LatencyCritical",
            "latency-critical",
            "low-latency",
            "backend-full"
        ),
        VirtualizationSchedulerTuning {
            threshold_divisor: 2,
            threshold_multiplier: 1,
            burst_divisor: 2,
            burst_multiplier: 1,
        }
    );
    assert_eq!(
        virtualization_scheduler_tuning("Background", "background", "throughput", "backend-basic"),
        VirtualizationSchedulerTuning {
            threshold_divisor: 1,
            threshold_multiplier: 2,
            burst_divisor: 1,
            burst_multiplier: 2,
        }
    );
    assert_eq!(
        virtualization_scheduler_tuning("Balanced", "balanced", "balanced", "backend-full"),
        VirtualizationSchedulerTuning {
            threshold_divisor: 1,
            threshold_multiplier: 1,
            burst_divisor: 1,
            burst_multiplier: 1,
        }
    );
}
