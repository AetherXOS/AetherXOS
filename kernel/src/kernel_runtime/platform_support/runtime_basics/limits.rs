pub(crate) fn log_core_runtime_limits() {
    let core_limits = aethercore::config::KernelConfig::runtime_limits();
    aethercore::klog_info!(
        "Core runtime limits: irq_base={} watchdog_ns={} rt_force_min={} rt_burst={} loader_segments={} loader_image_bytes={} launch_name_len={} launch_image_bytes={} launch_timeout={} vfs_mounts={} vfs_path={} rq_saturation={} mutex_spin_limit={}",
        core_limits.irq_vector_base,
        core_limits.watchdog_hard_stall_ns,
        core_limits.rt_force_reschedule_min_ticks,
        core_limits.rt_deadline_burst_threshold,
        core_limits.module_loader_max_load_segments,
        core_limits.module_loader_max_total_image_bytes,
        core_limits.launch_max_process_name_len,
        core_limits.launch_max_boot_image_bytes,
        core_limits.launch_handoff_stage_timeout_epochs,
        core_limits.vfs_max_mounts,
        core_limits.vfs_max_mount_path,
        core_limits.power_runqueue_saturation_limit,
        core_limits.irqsafe_mutex_deadlock_spin_limit
    );
}
