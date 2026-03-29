use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn pressure_thresholds_track_virtualization_latency_bias() {
    assert_eq!(
        pressure_level_thresholds(GOVERNOR_BIAS_BALANCED),
        (4, 9, 19)
    );
    assert_eq!(
        pressure_level_thresholds(GOVERNOR_BIAS_AGGRESSIVE),
        (6, 12, 24)
    );
    assert_eq!(pressure_level_thresholds(GOVERNOR_BIAS_RELAXED), (3, 7, 15));
}

#[test_case]
fn aggressive_bias_raises_pressure_sensitivity_window() {
    let (critical_max, medium_max, low_max) = pressure_level_thresholds(GOVERNOR_BIAS_AGGRESSIVE);
    assert!(critical_max > 4);
    assert!(medium_max > 9);
    assert!(low_max > 19);
}

#[test_case]
fn relaxed_bias_lowers_pressure_sensitivity_window() {
    let (critical_max, medium_max, low_max) = pressure_level_thresholds(GOVERNOR_BIAS_RELAXED);
    assert!(critical_max < 4);
    assert!(medium_max < 9);
    assert!(low_max < 19);
}
