use super::super::preset::{PRESET_APPLY_CALLS, PRESET_SET_CALLS};
use super::super::state::{
    DRIFT_EVENTS, DRIFT_REAPPLY_CALLS, DRIFT_REAPPLY_SUPPRESSED_COOLDOWN, DRIFT_SAMPLE_CALLS,
    LAST_DRIFT_REASON, LAST_DRIVER_WAIT_TIMEOUT_DELTA, LAST_REAPPLY_TICK,
};
use super::super::*;

pub fn runtime_policy_snapshot() -> CoreRuntimePolicySnapshot {
    let governor = current_virtualization_runtime_governor();
    let effective_execution =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let effective_governor =
        crate::config::KernelConfig::virtualization_effective_governor_profile();
    CoreRuntimePolicySnapshot {
        active_preset: runtime_policy_preset(),
        set_calls: PRESET_SET_CALLS.load(Ordering::Relaxed),
        apply_calls: PRESET_APPLY_CALLS.load(Ordering::Relaxed),
        drift_samples: DRIFT_SAMPLE_CALLS.load(Ordering::Relaxed),
        drift_events: DRIFT_EVENTS.load(Ordering::Relaxed),
        drift_reapply_calls: DRIFT_REAPPLY_CALLS.load(Ordering::Relaxed),
        drift_reapply_suppressed_cooldown: DRIFT_REAPPLY_SUPPRESSED_COOLDOWN
            .load(Ordering::Relaxed),
        drift_sample_interval_ticks:
            crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks(),
        drift_reapply_cooldown_ticks:
            crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks(),
        last_reapply_tick: LAST_REAPPLY_TICK.load(Ordering::Relaxed),
        last_drift_reason: LAST_DRIFT_REASON.load(Ordering::Relaxed) as u8,
        last_driver_wait_timeout_delta: LAST_DRIVER_WAIT_TIMEOUT_DELTA.load(Ordering::Relaxed),
        virtualization_execution_profile: effective_execution.scheduling_class.as_str(),
        virtualization_governor_profile: effective_governor.governor_class.as_str(),
        virtualization_governor_class: governor.governor_class,
        virtualization_latency_bias: governor.latency_bias,
    }
}
