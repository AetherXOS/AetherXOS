use super::*;

#[test_case]
fn virtualization_power_tuning_matches_execution_profiles() {
    assert_eq!(
        virtualization_power_tuning(
            "LatencyCritical",
            "latency-critical",
            "backend-full",
            "low-latency"
        ),
        VirtualizationPowerTuning {
            prefer_active_pstate: true,
            prefer_shallow_idle: true,
        }
    );
    assert_eq!(
        virtualization_power_tuning("Background", "background", "backend-basic", "throughput"),
        VirtualizationPowerTuning {
            prefer_active_pstate: false,
            prefer_shallow_idle: false,
        }
    );
    assert_eq!(
        virtualization_power_tuning("Balanced", "balanced", "backend-full", "balanced"),
        VirtualizationPowerTuning {
            prefer_active_pstate: false,
            prefer_shallow_idle: true,
        }
    );
}
