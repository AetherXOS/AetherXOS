#![cfg(target_os = "none")]

use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
    GOVERNOR_CLASS_BACKGROUND_OPTIMIZED, GOVERNOR_CLASS_BALANCED, GOVERNOR_CLASS_LATENCY_FOCUSED,
    GOVERNOR_ENERGY_BALANCED, GOVERNOR_ENERGY_PERFORMANCE, GOVERNOR_ENERGY_SAVING,
    RUNTIME_DISPATCH_BALANCED, RUNTIME_DISPATCH_CONSERVATIVE, RUNTIME_DISPATCH_LATENCY_SAFE,
    RUNTIME_DISPATCH_WINDOW_HOLD, RUNTIME_DISPATCH_WINDOW_SHORT, RUNTIME_PREEMPT_COOPERATIVE,
    RUNTIME_PREEMPT_HOLD, RUNTIME_PREEMPT_PREEMPTIBLE, RUNTIME_SCHED_LANE_BACKGROUND,
    RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
};

#[test_case]
fn effective_virtualization_contracts_hold() {
    assert!(virtualization_effective_execution_contract_holds());
    assert!(virtualization_effective_governor_contract_holds());
}

#[test_case]
fn dispatch_contract_accepts_known_pairs() {
    assert!(virtualization_dispatch_contract_holds(
        RUNTIME_DISPATCH_LATENCY_SAFE,
        RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
        RUNTIME_PREEMPT_PREEMPTIBLE,
        RUNTIME_DISPATCH_WINDOW_SHORT,
    ));
    assert!(virtualization_dispatch_contract_holds(
        RUNTIME_DISPATCH_CONSERVATIVE,
        RUNTIME_SCHED_LANE_BACKGROUND,
        RUNTIME_PREEMPT_HOLD,
        RUNTIME_DISPATCH_WINDOW_HOLD,
    ));
}

#[test_case]
fn dispatch_contract_rejects_mismatched_pairs() {
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
