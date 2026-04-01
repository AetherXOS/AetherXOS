use super::*;

#[test_case]
fn preset_roundtrip_is_stable() {
    set_runtime_policy_preset(CoreRuntimePolicyPreset::Realtime);
    assert_eq!(runtime_policy_preset(), CoreRuntimePolicyPreset::Realtime);
    set_runtime_policy_preset(CoreRuntimePolicyPreset::Server);
    assert_eq!(runtime_policy_preset(), CoreRuntimePolicyPreset::Server);
}

#[test_case]
fn reapply_cooldown_guard_behaves_as_expected() {
    let cooldown = crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks();
    assert!(!can_reapply_now(10, 10));
    assert!(!can_reapply_now(cooldown.saturating_sub(1), 0));
    assert!(can_reapply_now(cooldown, 0));
    assert!(can_reapply_now(cooldown.saturating_add(1), 1));
}

#[test_case]
fn drift_reason_name_roundtrip_is_stable() {
    assert_eq!(drift_reason_name(DriftReasonCode::None.as_u8()), "none");
    assert_eq!(
        drift_reason_name(DriftReasonCode::PressureHigh.as_u8()),
        "pressure_high"
    );
    assert_eq!(
        drift_reason_name(DriftReasonCode::DriverWaitTimeout.as_u8()),
        "driver_wait_timeout"
    );
}
