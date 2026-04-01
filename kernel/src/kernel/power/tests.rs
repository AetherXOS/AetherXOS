use super::*;
use core::sync::atomic::Ordering;

#[path = "tests/tuning.rs"]
mod tuning;

fn reset_for_tests() {
    IDLE_CALLS.store(0, Ordering::Relaxed);
    C1_ENTRIES.store(0, Ordering::Relaxed);
    C2_ENTRIES.store(0, Ordering::Relaxed);
    C3_ENTRIES.store(0, Ordering::Relaxed);
    PSTATE_SWITCHES.store(0, Ordering::Relaxed);
    CURRENT_PSTATE.store(1, Ordering::Relaxed);
    PSTATE_OVERRIDE.store(OVERRIDE_NONE, Ordering::Relaxed);
    PSTATE_OVERRIDE_SET_CALLS.store(0, Ordering::Relaxed);
    PSTATE_OVERRIDE_CLEAR_CALLS.store(0, Ordering::Relaxed);
    CSTATE_OVERRIDE.store(OVERRIDE_NONE, Ordering::Relaxed);
    CSTATE_OVERRIDE_SET_CALLS.store(0, Ordering::Relaxed);
    CSTATE_OVERRIDE_CLEAR_CALLS.store(0, Ordering::Relaxed);
    ACPI_PROFILE_LOADED.store(ACPI_PROFILE_DISABLED, Ordering::Relaxed);
    ACPI_FADT_REVISION.store(0, Ordering::Relaxed);
    POLICY_GUARD_HITS.store(0, Ordering::Relaxed);
    RUNQUEUE_CLAMP_EVENTS.store(0, Ordering::Relaxed);
    FAILSAFE_IDLE_FALLBACKS.store(0, Ordering::Relaxed);
    OVERRIDE_REJECTS_NO_ACPI.store(0, Ordering::Relaxed);
}

#[test_case]
fn no_acpi_rejects_deep_override_modes() {
    reset_for_tests();
    assert!(!set_pstate_override_guarded(PState::PowerSave));
    assert!(!set_cstate_override_guarded(CState::C3));

    let s = stats();
    assert!(s.policy_guard_hits >= 2);
    assert!(s.override_rejects_no_acpi >= 2);
}

#[test_case]
fn with_acpi_allows_deep_override_modes() {
    reset_for_tests();
    init_from_acpi(true, 3);
    assert!(set_pstate_override_guarded(PState::PowerSave));
    assert!(set_cstate_override_guarded(CState::C3));
    assert!(matches!(pstate_override(), Some(PState::PowerSave)));
    assert!(matches!(cstate_override(), Some(CState::C3)));
}

#[test_case]
fn idle_runqueue_hint_is_clamped() {
    reset_for_tests();
    let saturation_limit = crate::config::KernelConfig::power_runqueue_saturation_limit();
    let _ = on_idle(saturation_limit + 1024);
    let s = stats();
    assert!(s.runqueue_clamp_events >= 1);
}
