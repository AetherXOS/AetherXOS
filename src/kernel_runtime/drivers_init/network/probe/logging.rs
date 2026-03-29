use super::super::logging::{
    log_network_driver_failure_for, log_network_driver_initialized, log_network_probe_discovery,
    log_virtio_driver_runtime,
};

pub(super) fn log_network_probe_plan() {
    let plan = hypercore::modules::drivers::network_probe_plan();
    for step in plan {
        hypercore::klog_info!(
            "Network probe step: order={} name={} bus={:?} dep={:?} kind={:?}",
            step.order,
            step.name,
            step.bus,
            step.dependency,
            step.active_kind
        );
    }
}

pub(super) fn log_network_driver_policy() {
    use hypercore::modules::drivers::{
        network_driver_policy_snapshot, probe_policy_fallback_kind, probe_policy_primary_kind,
    };

    let policy = network_driver_policy_snapshot();
    hypercore::klog_info!(
        "Network driver policy: active={:?} set_calls={} primary={:?} fallback={:?} remediation={:?} remediation_set_calls={} slo_streak={} cooldown_base={} jitter_mask={:#x} rebind_before_failover={}",
        policy.active_policy,
        policy.set_calls,
        probe_policy_primary_kind(policy.active_policy),
        probe_policy_fallback_kind(policy.active_policy),
        policy.remediation_profile,
        policy.remediation_profile_set_calls,
        policy.remediation_tuning.breach_streak_threshold,
        policy.remediation_tuning.cooldown_base_samples,
        policy.remediation_tuning.cooldown_jitter_mask,
        policy.remediation_tuning.rebind_before_failover
    );
}

pub(super) fn log_driver_init_success(driver: &hypercore::modules::drivers::ProbedNetworkDriver) {
    log_network_driver_initialized(driver.name());
}

pub(super) fn log_driver_init_failure(driver: &hypercore::modules::drivers::ProbedNetworkDriver) {
    let status = driver.status();
    log_network_driver_failure_for(driver.name(), "initialization failed", &status);
}

pub(super) fn log_probe_discovery(driver: &hypercore::modules::drivers::ProbedNetworkDriver) {
    log_network_probe_discovery(driver);
}

pub(super) fn log_virtio_runtime(driver: &hypercore::modules::drivers::ProbedNetworkDriver) {
    if let hypercore::modules::drivers::ProbedNetworkDriver::VirtIo(net) = driver {
        log_virtio_driver_runtime(net);
    }
}
