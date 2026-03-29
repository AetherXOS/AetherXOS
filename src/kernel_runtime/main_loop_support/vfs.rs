#[cfg(feature = "vfs")]
pub(crate) fn service_vfs_runtime() {
    use core::sync::atomic::Ordering;

    let sample = super::super::VFS_SLO_SAMPLE_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);

    if sample % super::super::VFS_SLO_SAMPLE_INTERVAL == 0 {
        let report = hypercore::modules::vfs::evaluate_mount_health_slo();
        if report.breach_count > 0 {
            super::super::VFS_SLO_BREACH_STREAK.fetch_add(1, Ordering::Relaxed);
        } else {
            super::super::VFS_SLO_BREACH_STREAK.store(0, Ordering::Relaxed);
        }

        let log_period = super::super::VFS_SLO_SAMPLE_INTERVAL
            .saturating_mul(super::super::VFS_SLO_LOG_INTERVAL_MULTIPLIER);
        let last_log = super::super::VFS_SLO_LAST_LOG_SAMPLE.load(Ordering::Relaxed);
        let should_log_now = sample.saturating_sub(last_log) >= log_period;

        if report.breach_count > 0 && should_log_now {
            super::super::VFS_SLO_LAST_LOG_SAMPLE.store(sample, Ordering::Relaxed);
            hypercore::klog_warn!(
                "[VFS SLO] breaches={} read_p99={} write_p99={} mount_fail={}‰ unmount_fail={}‰ path_rejects={} capacity={}%% mounts={}",
                report.breach_count,
                report.read_latency_p99_ticks,
                report.write_latency_p99_ticks,
                report.mount_failure_rate_per_mille,
                report.unmount_failure_rate_per_mille,
                report.path_validation_failures,
                report.mount_capacity_percent,
                report.total_mounts
            );
        }

        let streak = super::super::VFS_SLO_BREACH_STREAK.load(Ordering::Relaxed);
        if streak >= super::super::VFS_SLO_ACTION_STREAK_THRESHOLD {
            match hypercore::modules::vfs::recommended_mount_health_action(report) {
                hypercore::modules::vfs::VfsMountHealthAction::PreferUnbufferedIo => {
                    if hypercore::config::KernelConfig::vfs_enable_buffered_io() {
                        hypercore::config::KernelConfig::set_vfs_enable_buffered_io(Some(false));
                        super::super::VFS_SLO_POLICY_ACTIONS.fetch_add(1, Ordering::Relaxed);
                        hypercore::klog_warn!(
                            "[VFS SLO] policy action: prefer_unbuffered_io (actions={})",
                            super::super::VFS_SLO_POLICY_ACTIONS.load(Ordering::Relaxed)
                        );
                    }
                }
                hypercore::modules::vfs::VfsMountHealthAction::PreferBufferedIo => {
                    if !hypercore::config::KernelConfig::vfs_enable_buffered_io() {
                        hypercore::config::KernelConfig::set_vfs_enable_buffered_io(Some(true));
                        super::super::VFS_SLO_POLICY_ACTIONS.fetch_add(1, Ordering::Relaxed);
                        hypercore::klog_warn!(
                            "[VFS SLO] policy action: prefer_buffered_io (actions={})",
                            super::super::VFS_SLO_POLICY_ACTIONS.load(Ordering::Relaxed)
                        );
                    }
                }
                hypercore::modules::vfs::VfsMountHealthAction::ThrottleMountChurn => {
                    if should_log_now {
                        hypercore::klog_warn!(
                            "[VFS SLO] policy action: throttle_mount_churn (advisory)"
                        );
                    }
                }
                hypercore::modules::vfs::VfsMountHealthAction::None => {}
            }
        }
    }
}
