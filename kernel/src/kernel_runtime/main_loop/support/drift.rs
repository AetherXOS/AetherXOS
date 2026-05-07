pub(crate) fn log_runtime_policy_drift(
    drift: aethercore::kernel::policy::CoreRuntimePolicyDriftReport,
) {
    if drift.drifted {
        aethercore::klog_warn!(
            "[POLICY DRIFT] preset={:?} reason={}({}) tick={} pressure={:?}/{:?} rt_starvation={} net_breaches={} vfs_breaches={} driver_wait_delta={} virt_exec={} virt_governor_profile={} virt_governor={} virt_latency_bias={} reapply_attempted={} reapply_executed={} reapply_suppressed={}",
            drift.preset,
            drift.reason_name,
            drift.reason,
            drift.sampled_tick,
            drift.pressure_class,
            drift.scheduler_class,
            drift.rt_starvation_alert,
            drift.network_slo_breaches,
            drift.vfs_slo_breaches,
            drift.driver_wait_timeout_delta,
            drift.virtualization_execution_profile,
            drift.virtualization_governor_profile,
            drift.virtualization_governor_class,
            drift.virtualization_latency_bias,
            drift.reapply_attempted,
            drift.reapply_executed,
            drift.reapply_suppressed_by_cooldown
        );
    }
}
