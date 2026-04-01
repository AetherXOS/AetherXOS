use super::*;
use core::sync::atomic::Ordering;

#[test_case]
fn startup_stage_ordering_violation_is_recorded() {
    STARTUP_TRANSITIONS.store(0, Ordering::Relaxed);
    STARTUP_ORDERING_VIOLATIONS.store(0, Ordering::Relaxed);
    STARTUP_LAST_STAGE.store(0, Ordering::Relaxed);

    mark_stage(StartupStage::DriversInit);
    mark_stage(StartupStage::HeapInit);

    let d = diagnostics();
    assert!(d.transitions >= 2);
    assert!(d.ordering_violations >= 1);
}
