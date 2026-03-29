use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::config::KernelConfig;
use crate::hal::common::virt::current_virtualization_scheduler_tuning;
#[cfg(test)]
use crate::hal::common::virt::{virtualization_scheduler_tuning, VirtualizationSchedulerTuning};
use crate::interfaces::SchedulerAction;

static TICKS: AtomicU64 = AtomicU64::new(0);
static RESCHEDULES: AtomicU64 = AtomicU64::new(0);
static FORCED_RESCHEDULES: AtomicU64 = AtomicU64::new(0);
static CONTINUE_STREAK: AtomicUsize = AtomicUsize::new(0);
static LAST_RUNQUEUE_LEN: AtomicUsize = AtomicUsize::new(0);
static STARVATION_ALERT: AtomicBool = AtomicBool::new(false);
static EDF_PRESSURE_EVENTS: AtomicU64 = AtomicU64::new(0);
static MANUAL_FORCE_REQUESTS: AtomicU64 = AtomicU64::new(0);
static FORCE_RESCHEDULE_ON_NEXT_TICK: AtomicBool = AtomicBool::new(false);
static FORCE_THRESHOLD_OVERRIDE_TICKS: AtomicUsize = AtomicUsize::new(0);
static DEADLINE_BURST_THRESHOLD: AtomicUsize = AtomicUsize::new(0);
static LAST_FORCED_TICK: AtomicU64 = AtomicU64::new(0);
static FORCED_BURST_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEADLINE_ALERT_ACTIVE: AtomicBool = AtomicBool::new(false);
static DEADLINE_ALERT_EVENTS: AtomicU64 = AtomicU64::new(0);
static MAX_CONTINUE_STREAK: AtomicUsize = AtomicUsize::new(0);
#[cfg(param_scheduler = "EDF")]
static LAST_EDF_DEADLINE_MISSES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct RtPreemptionStats {
    pub ticks: u64,
    pub reschedules: u64,
    pub forced_reschedules: u64,
    pub continue_streak: usize,
    pub last_runqueue_len: usize,
    pub starvation_alert: bool,
    pub edf_pressure_events: u64,
    pub manual_force_requests: u64,
    pub force_threshold_ticks: usize,
    pub force_threshold_override_ticks: usize,
    pub deadline_burst_threshold: usize,
    pub forced_burst_count: usize,
    pub deadline_alert_active: bool,
    pub deadline_alert_events: u64,
    pub max_continue_streak: usize,
}

fn force_threshold_ticks() -> usize {
    let min_ticks = KernelConfig::rt_force_reschedule_min_ticks();
    let override_ticks = FORCE_THRESHOLD_OVERRIDE_TICKS.load(Ordering::Relaxed);
    if override_ticks > 0 {
        return override_ticks.max(min_ticks);
    }

    let period = KernelConfig::rt_period_ns();
    let slice = KernelConfig::time_slice().max(1);
    let ratio = (period / slice).max(min_ticks as u64);
    usize::try_from(ratio).unwrap_or(8)
}

#[inline(always)]
fn deadline_burst_threshold() -> usize {
    let configured = KernelConfig::rt_deadline_burst_threshold();
    let raw = DEADLINE_BURST_THRESHOLD.load(Ordering::Relaxed);
    let base = if raw == 0 {
        configured
    } else {
        raw.max(configured)
    };
    let tuning = current_virtualization_scheduler_tuning();
    let reduced = base / tuning.burst_divisor.max(1);
    reduced
        .saturating_mul(tuning.burst_multiplier.max(1))
        .max(1)
}

fn update_max_continue_streak(streak: usize) {
    let mut observed = MAX_CONTINUE_STREAK.load(Ordering::Relaxed);
    while streak > observed {
        match MAX_CONTINUE_STREAK.compare_exchange_weak(
            observed,
            streak,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(next) => observed = next,
        }
    }
}

fn clear_alert_state() {
    STARVATION_ALERT.store(false, Ordering::Relaxed);
    DEADLINE_ALERT_ACTIVE.store(false, Ordering::Relaxed);
    FORCED_BURST_COUNT.store(0, Ordering::Relaxed);
}

fn record_forced_reschedule(now_tick: u64, base_threshold: usize) {
    let last = LAST_FORCED_TICK.swap(now_tick, Ordering::Relaxed);
    let burst_window = u64::try_from(base_threshold.max(2)).unwrap_or(2);
    let burst_count = if now_tick.saturating_sub(last) <= burst_window {
        FORCED_BURST_COUNT
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    } else {
        FORCED_BURST_COUNT.store(1, Ordering::Relaxed);
        1
    };

    let burst_threshold = deadline_burst_threshold();
    if burst_count >= burst_threshold {
        DEADLINE_ALERT_ACTIVE.store(true, Ordering::Relaxed);
        DEADLINE_ALERT_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(param_scheduler = "EDF")]
fn edf_deadline_pressure_divisor() -> usize {
    let stats = crate::modules::schedulers::edf::runtime_stats();
    let last = LAST_EDF_DEADLINE_MISSES.load(Ordering::Relaxed);
    if stats.deadline_misses > last {
        LAST_EDF_DEADLINE_MISSES.store(stats.deadline_misses, Ordering::Relaxed);
        EDF_PRESSURE_EVENTS.fetch_add(1, Ordering::Relaxed);
        2
    } else {
        1
    }
}

#[cfg(not(param_scheduler = "EDF"))]
fn edf_deadline_pressure_divisor() -> usize {
    1
}

pub fn on_scheduler_tick(action: &SchedulerAction, runqueue_len: usize) -> bool {
    let now_tick = TICKS.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    LAST_RUNQUEUE_LEN.store(runqueue_len, Ordering::Relaxed);

    if FORCE_RESCHEDULE_ON_NEXT_TICK.swap(false, Ordering::Relaxed) {
        FORCED_RESCHEDULES.fetch_add(1, Ordering::Relaxed);
        record_forced_reschedule(now_tick, force_threshold_ticks());
        CONTINUE_STREAK.store(0, Ordering::Relaxed);
        return true;
    }

    if *action == SchedulerAction::Reschedule {
        RESCHEDULES.fetch_add(1, Ordering::Relaxed);
        CONTINUE_STREAK.store(0, Ordering::Relaxed);
        clear_alert_state();
        return false;
    }

    let streak = CONTINUE_STREAK
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    update_max_continue_streak(streak);

    let base_threshold = force_threshold_ticks();
    let tuning = current_virtualization_scheduler_tuning();
    let mut threshold = (base_threshold / edf_deadline_pressure_divisor())
        .max(2)
        .saturating_div(tuning.threshold_divisor.max(1))
        .saturating_mul(tuning.threshold_multiplier.max(1))
        .max(2);
    if DEADLINE_ALERT_ACTIVE.load(Ordering::Relaxed) {
        threshold = (threshold / 2).max(2);
    }

    if runqueue_len > 1 && streak >= threshold {
        FORCED_RESCHEDULES.fetch_add(1, Ordering::Relaxed);
        STARVATION_ALERT.store(true, Ordering::Relaxed);
        record_forced_reschedule(now_tick, base_threshold);
        CONTINUE_STREAK.store(0, Ordering::Relaxed);
        return true;
    }

    if runqueue_len <= 1 {
        clear_alert_state();
    }

    false
}

pub fn on_context_switch() {
    CONTINUE_STREAK.store(0, Ordering::Relaxed);
    clear_alert_state();
}

pub fn request_forced_reschedule() {
    MANUAL_FORCE_REQUESTS.fetch_add(1, Ordering::Relaxed);
    FORCE_RESCHEDULE_ON_NEXT_TICK.store(true, Ordering::Relaxed);
}

pub fn set_force_threshold_override_ticks(override_ticks: Option<usize>) {
    let value = override_ticks.unwrap_or(0);
    FORCE_THRESHOLD_OVERRIDE_TICKS.store(value, Ordering::Relaxed);
}

pub fn set_deadline_burst_threshold(threshold: usize) {
    DEADLINE_BURST_THRESHOLD.store(
        threshold.max(KernelConfig::rt_deadline_burst_threshold()),
        Ordering::Relaxed,
    );
}

pub fn clear_deadline_alert() {
    clear_alert_state();
}

pub fn stats() -> RtPreemptionStats {
    let force_threshold_override_ticks = FORCE_THRESHOLD_OVERRIDE_TICKS.load(Ordering::Relaxed);
    let deadline_burst_threshold = deadline_burst_threshold();
    RtPreemptionStats {
        ticks: TICKS.load(Ordering::Relaxed),
        reschedules: RESCHEDULES.load(Ordering::Relaxed),
        forced_reschedules: FORCED_RESCHEDULES.load(Ordering::Relaxed),
        continue_streak: CONTINUE_STREAK.load(Ordering::Relaxed),
        last_runqueue_len: LAST_RUNQUEUE_LEN.load(Ordering::Relaxed),
        starvation_alert: STARVATION_ALERT.load(Ordering::Relaxed),
        edf_pressure_events: EDF_PRESSURE_EVENTS.load(Ordering::Relaxed),
        manual_force_requests: MANUAL_FORCE_REQUESTS.load(Ordering::Relaxed),
        force_threshold_ticks: force_threshold_ticks(),
        force_threshold_override_ticks,
        deadline_burst_threshold,
        forced_burst_count: FORCED_BURST_COUNT.load(Ordering::Relaxed),
        deadline_alert_active: DEADLINE_ALERT_ACTIVE.load(Ordering::Relaxed),
        deadline_alert_events: DEADLINE_ALERT_EVENTS.load(Ordering::Relaxed),
        max_continue_streak: MAX_CONTINUE_STREAK.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests;
