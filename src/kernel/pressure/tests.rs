use super::*;

#[path = "tests/tuning.rs"]
mod tuning;

#[test_case]
fn pressure_classification_thresholds_are_stable() {
    assert_eq!(
        classify_pressure(1, 5, false, 0, "balanced"),
        CorePressureClass::Nominal
    );
    assert_eq!(
        classify_pressure(9, 40, false, 0, "balanced"),
        CorePressureClass::Elevated
    );
    assert_eq!(
        classify_pressure(4, 81, false, 0, "balanced"),
        CorePressureClass::High
    );
    assert_eq!(
        classify_pressure(2, 10, false, 1, "balanced"),
        CorePressureClass::Critical
    );
}

#[test_case]
fn scheduler_pressure_classification_thresholds_are_stable() {
    assert_eq!(
        classify_scheduler_pressure(1, 1, 0, false, "balanced"),
        SchedulerPressureClass::Nominal
    );
    assert_eq!(
        classify_scheduler_pressure(8, 2, 0, false, "balanced"),
        SchedulerPressureClass::Elevated
    );
    assert_eq!(
        classify_scheduler_pressure(4, 9, 0, false, "balanced"),
        SchedulerPressureClass::High
    );
    assert_eq!(
        classify_scheduler_pressure(2, 5, 1, false, "balanced"),
        SchedulerPressureClass::Critical
    );
}
