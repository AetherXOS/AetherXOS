pub(crate) fn log_library_surfaces(verbose_inventory: bool) {
    hypercore::klog_info!(
        "Library surfaces: vfs={} network={} ipc={} verbose_inventory={}",
        hypercore::config::KernelConfig::is_vfs_library_api_exposed(),
        hypercore::config::KernelConfig::is_network_library_api_exposed(),
        hypercore::config::KernelConfig::is_ipc_library_api_exposed(),
        verbose_inventory,
    );
}
