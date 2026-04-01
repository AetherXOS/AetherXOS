use super::*;
use core::sync::atomic::Ordering;

#[path = "tests/tuning.rs"]
mod tuning;

fn reset_for_tests() {
    TICKS.store(0, Ordering::Relaxed);
    RESCHEDULES.store(0, Ordering::Relaxed);
    FORCED_RESCHEDULES.store(0, Ordering::Relaxed);
    CONTINUE_STREAK.store(0, Ordering::Relaxed);
    LAST_RUNQUEUE_LEN.store(0, Ordering::Relaxed);
    STARVATION_ALERT.store(false, Ordering::Relaxed);
    EDF_PRESSURE_EVENTS.store(0, Ordering::Relaxed);
    MANUAL_FORCE_REQUESTS.store(0, Ordering::Relaxed);
    FORCE_RESCHEDULE_ON_NEXT_TICK.store(false, Ordering::Relaxed);
    FORCE_THRESHOLD_OVERRIDE_TICKS.store(0, Ordering::Relaxed);
    DEADLINE_BURST_THRESHOLD.store(0, Ordering::Relaxed);
    LAST_FORCED_TICK.store(0, Ordering::Relaxed);
    FORCED_BURST_COUNT.store(0, Ordering::Relaxed);
    DEADLINE_ALERT_ACTIVE.store(false, Ordering::Relaxed);
    DEADLINE_ALERT_EVENTS.store(0, Ordering::Relaxed);
    MAX_CONTINUE_STREAK.store(0, Ordering::Relaxed);
}

#[test_case]
fn forced_reschedule_burst_raises_deadline_alert() {
    reset_for_tests();
    set_force_threshold_override_ticks(Some(2));
    set_deadline_burst_threshold(2);

    for _ in 0..8 {
        let _ = on_scheduler_tick(&SchedulerAction::Continue, 4);
    }

    let stats = stats();
    assert!(stats.forced_reschedules >= 2);
    assert!(stats.deadline_alert_events >= 1);
    assert!(stats.deadline_alert_active);
}

#[test_case]
fn manual_force_request_is_accounted() {
    reset_for_tests();
    request_forced_reschedule();
    assert!(on_scheduler_tick(&SchedulerAction::Continue, 2));

    let stats = stats();
    assert_eq!(stats.manual_force_requests, 1);
    assert!(stats.forced_reschedules >= 1);
}

#[test_case]
fn context_switch_clears_alert_state() {
    reset_for_tests();
    set_force_threshold_override_ticks(Some(2));
    set_deadline_burst_threshold(2);

    for _ in 0..8 {
        let _ = on_scheduler_tick(&SchedulerAction::Continue, 4);
    }
    on_context_switch();

    let stats = stats();
    assert!(!stats.deadline_alert_active);
    assert!(!stats.starvation_alert);
    assert_eq!(stats.continue_streak, 0);
}
