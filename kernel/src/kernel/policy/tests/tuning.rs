use super::*;
use crate::hal::common::virt::{GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED};

#[test_case]
fn aggressive_bias_tightens_drift_limits() {
    let thresholds = drift_threshold_profile(
        CoreRuntimePolicyPreset::Interactive,
        GOVERNOR_BIAS_AGGRESSIVE,
    );
    assert_eq!(
        governor_adjusted_drift_limit(2, GOVERNOR_BIAS_AGGRESSIVE),
        1
    );
    assert_eq!(
        governor_adjusted_driver_wait_limit(2, GOVERNOR_BIAS_AGGRESSIVE),
        1
    );
    assert_eq!(
        thresholds.pressure_class_threshold,
        crate::kernel::pressure::CorePressureClass::Elevated
    );
    assert!(pressure_exceeds_threshold(
        crate::kernel::pressure::CorePressureClass::Elevated,
        thresholds.pressure_class_threshold,
    ));
}

#[test_case]
fn relaxed_bias_relaxes_drift_limits() {
    let thresholds =
        drift_threshold_profile(CoreRuntimePolicyPreset::Interactive, GOVERNOR_BIAS_RELAXED);
    assert_eq!(governor_adjusted_drift_limit(1, GOVERNOR_BIAS_RELAXED), 2);
    assert_eq!(
        governor_adjusted_driver_wait_limit(1, GOVERNOR_BIAS_RELAXED),
        2
    );
    assert_eq!(
        thresholds.pressure_class_threshold,
        crate::kernel::pressure::CorePressureClass::High
    );
    assert!(!pressure_exceeds_threshold(
        crate::kernel::pressure::CorePressureClass::Elevated,
        thresholds.pressure_class_threshold,
    ));
}
