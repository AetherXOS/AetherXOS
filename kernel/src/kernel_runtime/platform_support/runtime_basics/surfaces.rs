pub(crate) fn log_library_surfaces(verbose_inventory: bool) {
    aethercore::klog_info!(
        "Library surfaces: vfs={} network={} ipc={} verbose_inventory={}",
        aethercore::config::KernelConfig::is_vfs_library_api_exposed(),
        aethercore::config::KernelConfig::is_network_library_api_exposed(),
        aethercore::config::KernelConfig::is_ipc_library_api_exposed(),
        verbose_inventory,
    );
}
