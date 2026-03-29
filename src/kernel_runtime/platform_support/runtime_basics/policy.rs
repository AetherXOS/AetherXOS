pub(crate) fn log_boundary_policy() {
    let boundary = hypercore::config::KernelConfig::boundary_mode();
    hypercore::klog_info!(
        "Boundary policy: mode={:?} core_minimal={} strict_optional={} library_max_services={}",
        boundary,
        hypercore::config::KernelConfig::is_core_minimal_enforced(),
        hypercore::config::KernelConfig::is_strict_optional_features_enabled(),
        hypercore::config::KernelConfig::library_max_services(),
    );
}

pub(crate) fn log_watchdog_policy() {
    hypercore::klog_info!(
        "Watchdog policy: soft_enabled={} soft_stall_ticks={} soft_action={}",
        hypercore::config::KernelConfig::is_soft_watchdog_enabled(),
        hypercore::config::KernelConfig::soft_watchdog_stall_ticks(),
        hypercore::config::KernelConfig::soft_watchdog_action(),
    );
}
