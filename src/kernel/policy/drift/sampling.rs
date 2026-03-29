use super::super::evaluation::evaluate_drift;
use super::super::sources::{
    driver_wait_timeout_delta, network_slo_breach_count, vfs_slo_breach_count,
};
use super::super::state::{
    DRIFT_EVENTS, DRIFT_REAPPLY_CALLS, DRIFT_REAPPLY_SUPPRESSED_COOLDOWN, DRIFT_SAMPLE_CALLS,
    LAST_DRIFT_REASON, LAST_DRIFT_SAMPLED_TICK, LAST_DRIVER_WAIT_TIMEOUT_DELTA, LAST_REAPPLY_TICK,
};
use super::super::*;

#[inline(always)]
pub(crate) fn can_reapply_now(now_tick: u64, last_reapply_tick: u64) -> bool {
    now_tick.saturating_sub(last_reapply_tick)
        >= crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks()
}

pub fn sample_policy_drift_if_due() -> Option<CoreRuntimePolicyDriftReport> {
    let now_tick = crate::kernel::watchdog::global_tick();
    let sample_interval = crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks();
    let last = LAST_DRIFT_SAMPLED_TICK.load(Ordering::Relaxed);
    if now_tick.saturating_sub(last) < sample_interval {
        return None;
    }

    LAST_DRIFT_SAMPLED_TICK.store(now_tick, Ordering::Relaxed);
    DRIFT_SAMPLE_CALLS.fetch_add(1, Ordering::Relaxed);

    let preset = runtime_policy_preset();
    let pressure = crate::kernel::pressure::snapshot();
    let governor = current_virtualization_runtime_governor();
    let effective_execution =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let effective_governor =
        crate::config::KernelConfig::virtualization_effective_governor_profile();
    let network_breaches = network_slo_breach_count();
    let vfs_breaches = vfs_slo_breach_count();
    let driver_wait_delta = driver_wait_timeout_delta();
    LAST_DRIVER_WAIT_TIMEOUT_DELTA.store(driver_wait_delta, Ordering::Relaxed);
    let (drifted, reason) = evaluate_drift(
        preset,
        pressure,
        network_breaches,
        vfs_breaches,
        driver_wait_delta,
    );
    let mut reapply_attempted = false;
    let mut reapply_executed = false;
    let mut reapply_suppressed_by_cooldown = false;
    if drifted {
        DRIFT_EVENTS.fetch_add(1, Ordering::Relaxed);
        reapply_attempted = true;
        let last_reapply_tick = LAST_REAPPLY_TICK.load(Ordering::Relaxed);
        if can_reapply_now(now_tick, last_reapply_tick) {
            DRIFT_REAPPLY_CALLS.fetch_add(1, Ordering::Relaxed);
            LAST_REAPPLY_TICK.store(now_tick, Ordering::Relaxed);
            apply_runtime_policy_preset();
            reapply_executed = true;
        } else {
            DRIFT_REAPPLY_SUPPRESSED_COOLDOWN.fetch_add(1, Ordering::Relaxed);
            reapply_suppressed_by_cooldown = true;
        }
    }
    LAST_DRIFT_REASON.store(reason.as_u8() as u64, Ordering::Relaxed);

    Some(CoreRuntimePolicyDriftReport {
        sampled_tick: now_tick,
        preset,
        drifted,
        reason: reason.as_u8(),
        reason_name: reason.name(),
        pressure_class: pressure.class,
        scheduler_class: pressure.scheduler_class,
        rt_starvation_alert: pressure.rt_starvation_alert,
        network_slo_breaches: network_breaches,
        vfs_slo_breaches: vfs_breaches,
        driver_wait_timeout_delta: driver_wait_delta,
        virtualization_execution_profile: effective_execution.scheduling_class.as_str(),
        virtualization_governor_profile: effective_governor.governor_class.as_str(),
        virtualization_governor_class: governor.governor_class,
        virtualization_latency_bias: governor.latency_bias,
        reapply_attempted,
        reapply_executed,
        reapply_suppressed_by_cooldown,
    })
}
