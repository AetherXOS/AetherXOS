pub(crate) fn log_boundary_policy() {
    let boundary = aethercore::config::KernelConfig::boundary_mode();
    aethercore::klog_info!(
        "Boundary policy: mode={:?} core_minimal={} strict_optional={} library_max_services={}",
        boundary,
        aethercore::config::KernelConfig::is_core_minimal_enforced(),
        aethercore::config::KernelConfig::is_strict_optional_features_enabled(),
        aethercore::config::KernelConfig::library_max_services(),
    );
}

pub(crate) fn log_watchdog_policy() {
    aethercore::klog_info!(
        "Watchdog policy: soft_enabled={} soft_stall_ticks={} soft_action={}",
        aethercore::config::KernelConfig::is_soft_watchdog_enabled(),
        aethercore::config::KernelConfig::soft_watchdog_stall_ticks(),
        aethercore::config::KernelConfig::soft_watchdog_action(),
    );
}
