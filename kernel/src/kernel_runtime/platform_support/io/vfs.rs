#[cfg(feature = "vfs")]
pub(crate) fn log_vfs_slo_thresholds() {
    let vfs_slo = hypercore::modules::vfs::mount_slo_thresholds();
    hypercore::klog_info!(
        "VFS SLO thresholds: read_p99<={}t write_p99<={}t mount_fail<={}‰ unmount_fail<={}‰ path_rejects<={} capacity<={}%%",
        vfs_slo.max_read_latency_ticks,
        vfs_slo.max_write_latency_ticks,
        vfs_slo.max_mount_failure_rate_per_mille,
        vfs_slo.max_unmount_failure_rate_per_mille,
        vfs_slo.max_path_validation_failures,
        vfs_slo.max_mount_capacity_percent
    );
}

#[cfg(all(feature = "vfs", feature = "vfs_telemetry"))]
pub(crate) fn log_vfs_core_runtime() {
    let vfs_bridge = hypercore::modules::vfs::bridge_stats();
    let vfs_mount = hypercore::kernel::vfs_control::stats();
    hypercore::klog_info!(
        "VFS core: mounts={}/{} failures={} unmount={}/{} unmount_failures={} unmount_by_path={}/{} unmount_by_path_failures={} path_rejects={} initrd_loads={} initrd_files={} initrd_bytes={} initrd_failures={} last_mount={} ramfs_open={} ramfs_create={} ramfs_remove={} fatfs_probe={} disk_read_calls={} disk_read_avg={} disk_read_p50={} disk_read_p95={} disk_read_p99={} disk_read_max={} disk_write_calls={} disk_write_avg={} disk_write_p50={} disk_write_p95={} disk_write_p99={} disk_write_max={}",
        vfs_mount.mount_success,
        vfs_mount.mount_attempts,
        vfs_mount.mount_failures,
        vfs_mount.unmount_success,
        vfs_mount.unmount_attempts,
        vfs_mount.unmount_failures,
        vfs_mount.unmount_by_path_success,
        vfs_mount.unmount_by_path_attempts,
        vfs_mount.unmount_by_path_failures,
        vfs_mount.path_validation_failures,
        vfs_mount.initrd_load_calls,
        vfs_mount.initrd_load_files,
        vfs_mount.initrd_load_bytes,
        vfs_mount.initrd_load_failures,
        vfs_mount.last_mount_id,
        vfs_bridge.ramfs_open_calls,
        vfs_bridge.ramfs_create_calls,
        vfs_bridge.ramfs_remove_calls,
        vfs_bridge.fatfs_bridge_probes,
        vfs_bridge.disk_read_calls,
        vfs_bridge.disk_read_avg_ticks,
        vfs_bridge.disk_read_latency_p50_ticks,
        vfs_bridge.disk_read_latency_p95_ticks,
        vfs_bridge.disk_read_latency_p99_ticks,
        vfs_bridge.disk_read_latency_max_ticks,
        vfs_bridge.disk_write_calls,
        vfs_bridge.disk_write_avg_ticks,
        vfs_bridge.disk_write_latency_p50_ticks,
        vfs_bridge.disk_write_latency_p95_ticks,
        vfs_bridge.disk_write_latency_p99_ticks,
        vfs_bridge.disk_write_latency_max_ticks
    );
}

#[cfg(all(feature = "vfs", not(feature = "vfs_telemetry")))]
pub(crate) fn log_vfs_core_runtime() {
    let vfs_mount = hypercore::kernel::vfs_control::stats();
    hypercore::klog_info!(
        "VFS core: mounts={}/{} failures={} unmount={}/{} unmount_failures={} unmount_by_path={}/{} unmount_by_path_failures={} path_rejects={} initrd_loads={} initrd_files={} initrd_bytes={} initrd_failures={} last_mount={}",
        vfs_mount.mount_success,
        vfs_mount.mount_attempts,
        vfs_mount.mount_failures,
        vfs_mount.unmount_success,
        vfs_mount.unmount_attempts,
        vfs_mount.unmount_failures,
        vfs_mount.unmount_by_path_success,
        vfs_mount.unmount_by_path_attempts,
        vfs_mount.unmount_by_path_failures,
        vfs_mount.path_validation_failures,
        vfs_mount.initrd_load_calls,
        vfs_mount.initrd_load_files,
        vfs_mount.initrd_load_bytes,
        vfs_mount.initrd_load_failures,
        vfs_mount.last_mount_id
    );
}

#[cfg(feature = "vfs")]
pub(crate) fn log_vfs_library_inventory() {
    #[cfg(feature = "vfs_library_backends")]
    {
        let backends = hypercore::modules::vfs::library_backend_inventory();
        if backends.is_empty() {
            hypercore::klog_info!("VFS library backends: none enabled");
        } else {
            for backend in backends {
                hypercore::klog_info!(
                    "VFS library backend: name={} feature={} target={} maturity={}",
                    backend.name,
                    backend.feature,
                    backend.target_support,
                    backend.maturity
                );
            }
        }
    }

    #[cfg(not(feature = "vfs_library_backends"))]
    {
        hypercore::klog_info!("VFS library backends: feature disabled");
    }
}
