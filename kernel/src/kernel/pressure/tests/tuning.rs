use super::*;
use crate::hal::common::virt::{GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED};

#[test_case]
fn aggressive_bias_lowers_pressure_thresholds() {
    assert!(
        governor_adjusted_threshold(RUNQUEUE_HIGH_THRESHOLD, GOVERNOR_BIAS_AGGRESSIVE)
            < RUNQUEUE_HIGH_THRESHOLD
    );
    assert_eq!(
        classify_pressure(12, 40, false, 0, GOVERNOR_BIAS_AGGRESSIVE),
        CorePressureClass::High
    );
}

#[test_case]
fn relaxed_bias_raises_pressure_thresholds() {
    assert!(
        governor_adjusted_threshold(RUNQUEUE_HIGH_THRESHOLD, GOVERNOR_BIAS_RELAXED)
            > RUNQUEUE_HIGH_THRESHOLD
    );
    assert_eq!(
        classify_scheduler_pressure(12, 6, 0, false, GOVERNOR_BIAS_RELAXED),
        SchedulerPressureClass::Elevated
    );
}
