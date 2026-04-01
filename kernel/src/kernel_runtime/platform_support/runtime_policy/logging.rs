use super::snapshot::current_runtime_policy_log_snapshot;

pub(crate) fn log_runtime_policy_summary() {
    let policy = current_runtime_policy_log_snapshot();
    hypercore::klog_info!(
        "Runtime policy: preset={:?} set_calls={} apply_calls={} drift_samples={} drift_events={} drift_reapply={} reapply_suppressed={} sample_interval_ticks={} reapply_cooldown_ticks={} last_reapply_tick={} last_reason={}({}) last_driver_wait_delta={} virt_exec={} virt_governor_profile={} virt_governor={} virt_latency_bias={}",
        policy.active_preset,
        policy.set_calls,
        policy.apply_calls,
        policy.drift_samples,
        policy.drift_events,
        policy.drift_reapply_calls,
        policy.drift_reapply_suppressed_cooldown,
        policy.drift_sample_interval_ticks,
        policy.drift_reapply_cooldown_ticks,
        policy.last_reapply_tick,
        policy.last_reason_name,
        policy.last_reason_code,
        policy.last_driver_wait_timeout_delta,
        policy.virtualization_execution_profile,
        policy.virtualization_governor_profile,
        policy.virtualization_governor_class,
        policy.virtualization_latency_bias
    );
}
