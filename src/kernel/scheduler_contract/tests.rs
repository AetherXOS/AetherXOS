use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
    GOVERNOR_CLASS_BACKGROUND_OPTIMIZED, GOVERNOR_CLASS_BALANCED, GOVERNOR_CLASS_LATENCY_FOCUSED,
    GOVERNOR_ENERGY_BALANCED, GOVERNOR_ENERGY_PERFORMANCE, GOVERNOR_ENERGY_SAVING,
    RUNTIME_DISPATCH_BALANCED, RUNTIME_DISPATCH_CONSERVATIVE, RUNTIME_DISPATCH_LATENCY_SAFE,
    RUNTIME_DISPATCH_WINDOW_HOLD, RUNTIME_DISPATCH_WINDOW_SHORT, RUNTIME_PREEMPT_COOPERATIVE,
    RUNTIME_PREEMPT_HOLD, RUNTIME_PREEMPT_PREEMPTIBLE, RUNTIME_SCHED_LANE_BACKGROUND,
    RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
};
use crate::kernel::virtualization_contract::{
    execution_profile_matches_status, expected_runtime_governor_class,
    virtualization_dispatch_contract_holds, virtualization_governor_bias_contract_holds,
};

#[test_case]
fn dispatch_contract_maps_latency_safe_to_critical_short_window() {
    assert!(virtualization_dispatch_contract_holds(
        RUNTIME_DISPATCH_LATENCY_SAFE,
        RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
        RUNTIME_PREEMPT_PREEMPTIBLE,
        RUNTIME_DISPATCH_WINDOW_SHORT,
    ));
}

#[test_case]
fn dispatch_contract_maps_conservative_to_background_hold_window() {
    assert!(virtualization_dispatch_contract_holds(
        RUNTIME_DISPATCH_CONSERVATIVE,
        RUNTIME_SCHED_LANE_BACKGROUND,
        RUNTIME_PREEMPT_HOLD,
        RUNTIME_DISPATCH_WINDOW_HOLD,
    ));
}

#[test_case]
fn dispatch_contract_rejects_mismatched_lane_and_window() {
    assert!(!virtualization_dispatch_contract_holds(
        RUNTIME_DISPATCH_BALANCED,
        RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
        RUNTIME_PREEMPT_COOPERATIVE,
        RUNTIME_DISPATCH_WINDOW_SHORT,
    ));
}

#[test_case]
fn governor_bias_contract_accepts_known_profiles() {
    assert!(virtualization_governor_bias_contract_holds(
        GOVERNOR_CLASS_LATENCY_FOCUSED,
        GOVERNOR_BIAS_AGGRESSIVE,
        GOVERNOR_ENERGY_PERFORMANCE,
    ));
    assert!(virtualization_governor_bias_contract_holds(
        GOVERNOR_CLASS_BALANCED,
        GOVERNOR_BIAS_BALANCED,
        GOVERNOR_ENERGY_BALANCED,
    ));
    assert!(virtualization_governor_bias_contract_holds(
        GOVERNOR_CLASS_BACKGROUND_OPTIMIZED,
        GOVERNOR_BIAS_RELAXED,
        GOVERNOR_ENERGY_SAVING,
    ));
}

#[test_case]
fn governor_bias_contract_rejects_inconsistent_energy_pairings() {
    assert!(!virtualization_governor_bias_contract_holds(
        GOVERNOR_CLASS_LATENCY_FOCUSED,
        GOVERNOR_BIAS_AGGRESSIVE,
        GOVERNOR_ENERGY_SAVING,
    ));
    assert!(!virtualization_governor_bias_contract_holds(
        GOVERNOR_CLASS_BACKGROUND_OPTIMIZED,
        GOVERNOR_BIAS_RELAXED,
        GOVERNOR_ENERGY_PERFORMANCE,
    ));
}

#[test_case]
fn expected_runtime_governor_class_maps_config_profile() {
    assert_eq!(
        expected_runtime_governor_class(crate::config::VirtualizationGovernorClass::Performance),
        crate::hal::common::virt::GOVERNOR_CLASS_PERFORMANCE
    );
    assert_eq!(
        expected_runtime_governor_class(crate::config::VirtualizationGovernorClass::Balanced),
        crate::hal::common::virt::GOVERNOR_CLASS_BALANCED
    );
    assert_eq!(
        expected_runtime_governor_class(crate::config::VirtualizationGovernorClass::Efficiency),
        crate::hal::common::virt::GOVERNOR_CLASS_EFFICIENCY
    );
}

#[test_case]
fn execution_profile_match_uses_effective_profile_name() {
    assert!(execution_profile_matches_status(
        crate::config::VirtualizationExecutionClass::LatencyCritical,
        "LatencyCritical",
    ));
    assert!(!execution_profile_matches_status(
        crate::config::VirtualizationExecutionClass::Background,
        "Balanced",
    ));
}
