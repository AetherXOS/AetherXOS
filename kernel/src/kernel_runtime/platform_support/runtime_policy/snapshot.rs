#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RuntimePolicyLogSnapshot {
    pub(crate) active_preset: aethercore::kernel::policy::CoreRuntimePolicyPreset,
    pub(crate) set_calls: u64,
    pub(crate) apply_calls: u64,
    pub(crate) drift_samples: u64,
    pub(crate) drift_events: u64,
    pub(crate) drift_reapply_calls: u64,
    pub(crate) drift_reapply_suppressed_cooldown: u64,
    pub(crate) drift_sample_interval_ticks: u64,
    pub(crate) drift_reapply_cooldown_ticks: u64,
    pub(crate) last_reapply_tick: u64,
    pub(crate) last_reason_code: u8,
    pub(crate) last_reason_name: &'static str,
    pub(crate) last_driver_wait_timeout_delta: u64,
    pub(crate) virtualization_execution_profile: &'static str,
    pub(crate) virtualization_governor_profile: &'static str,
    pub(crate) virtualization_governor_class: &'static str,
    pub(crate) virtualization_latency_bias: &'static str,
}

pub(crate) fn current_runtime_policy_log_snapshot() -> RuntimePolicyLogSnapshot {
    let snapshot = aethercore::kernel::policy::runtime_policy_snapshot();
    RuntimePolicyLogSnapshot {
        active_preset: snapshot.active_preset,
        set_calls: snapshot.set_calls,
        apply_calls: snapshot.apply_calls,
        drift_samples: snapshot.drift_samples,
        drift_events: snapshot.drift_events,
        drift_reapply_calls: snapshot.drift_reapply_calls,
        drift_reapply_suppressed_cooldown: snapshot.drift_reapply_suppressed_cooldown,
        drift_sample_interval_ticks: snapshot.drift_sample_interval_ticks,
        drift_reapply_cooldown_ticks: snapshot.drift_reapply_cooldown_ticks,
        last_reapply_tick: snapshot.last_reapply_tick,
        last_reason_code: snapshot.last_drift_reason,
        last_reason_name: aethercore::kernel::policy::drift_reason_name(snapshot.last_drift_reason),
        last_driver_wait_timeout_delta: snapshot.last_driver_wait_timeout_delta,
        virtualization_execution_profile: snapshot.virtualization_execution_profile,
        virtualization_governor_profile: snapshot.virtualization_governor_profile,
        virtualization_governor_class: snapshot.virtualization_governor_class,
        virtualization_latency_bias: snapshot.virtualization_latency_bias,
    }
}
