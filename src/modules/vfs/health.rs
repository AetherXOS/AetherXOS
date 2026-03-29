#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsMountHealthAction {
    None,
    PreferUnbufferedIo,
    PreferBufferedIo,
    ThrottleMountChurn,
}

#[derive(Debug, Clone, Copy)]
pub struct VfsMountSloThresholds {
    pub max_read_latency_ticks: u64,
    pub max_write_latency_ticks: u64,
    pub max_mount_failure_rate_per_mille: u64,
    pub max_unmount_failure_rate_per_mille: u64,
    pub max_path_validation_failures: u64,
    pub max_mount_capacity_percent: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct VfsMountSloReport {
    pub read_latency_p99_ticks: u64,
    pub write_latency_p99_ticks: u64,
    pub mount_failure_rate_per_mille: u64,
    pub unmount_failure_rate_per_mille: u64,
    pub path_validation_failures: u64,
    pub total_mounts: usize,
    pub mount_capacity_percent: usize,
    pub read_latency_breach: bool,
    pub write_latency_breach: bool,
    pub mount_failure_rate_breach: bool,
    pub unmount_failure_rate_breach: bool,
    pub path_validation_breach: bool,
    pub mount_capacity_breach: bool,
    pub breach_count: u8,
}

fn ms_to_watchdog_ticks(ms: u64) -> u64 {
    let tick_ns = crate::config::KernelConfig::time_slice().max(1);
    let target_ns = ms.saturating_mul(1_000_000).max(1);
    target_ns
        .saturating_add(tick_ns.saturating_sub(1))
        .saturating_div(tick_ns)
        .max(1)
}

pub fn mount_slo_thresholds() -> VfsMountSloThresholds {
    let latency_ticks = ms_to_watchdog_ticks(crate::config::KernelConfig::vfs_health_slo_ms());
    VfsMountSloThresholds {
        max_read_latency_ticks: latency_ticks,
        max_write_latency_ticks: latency_ticks,
        max_mount_failure_rate_per_mille:
            crate::config::KernelConfig::vfs_health_max_mount_failure_rate_per_mille(),
        max_unmount_failure_rate_per_mille:
            crate::config::KernelConfig::vfs_health_max_unmount_failure_rate_per_mille(),
        max_path_validation_failures:
            crate::config::KernelConfig::vfs_health_max_path_validation_failures(),
        max_mount_capacity_percent:
            crate::config::KernelConfig::vfs_health_max_mount_capacity_percent(),
    }
}

#[inline(always)]
fn failure_rate_per_mille(failures: u64, attempts: u64) -> u64 {
    if attempts == 0 {
        0
    } else {
        failures.saturating_mul(1000).saturating_div(attempts)
    }
}

pub fn evaluate_mount_health_slo() -> VfsMountSloReport {
    let thresholds = mount_slo_thresholds();
    let mount = crate::kernel::vfs_control::stats();

    #[cfg(feature = "vfs_telemetry")]
    let (read_p99_ticks, write_p99_ticks) = {
        let lat = crate::modules::vfs::telemetry::disk_io_latency_stats();
        (lat.read_p99_ticks, lat.write_p99_ticks)
    };
    #[cfg(not(feature = "vfs_telemetry"))]
    let (read_p99_ticks, write_p99_ticks) = (0u64, 0u64);

    let mount_failure_rate = failure_rate_per_mille(mount.mount_failures, mount.mount_attempts);
    let unmount_failure_rate =
        failure_rate_per_mille(mount.unmount_failures, mount.unmount_attempts);
    let max_mounts = crate::config::KernelConfig::vfs_max_mounts().max(1);
    let mount_capacity_percent = mount
        .total_mounts
        .saturating_mul(100)
        .saturating_div(max_mounts);

    let read_latency_breach = read_p99_ticks > thresholds.max_read_latency_ticks;
    let write_latency_breach = write_p99_ticks > thresholds.max_write_latency_ticks;
    let mount_failure_rate_breach =
        mount_failure_rate > thresholds.max_mount_failure_rate_per_mille;
    let unmount_failure_rate_breach =
        unmount_failure_rate > thresholds.max_unmount_failure_rate_per_mille;
    let path_validation_breach =
        mount.path_validation_failures > thresholds.max_path_validation_failures;
    let mount_capacity_breach = mount_capacity_percent > thresholds.max_mount_capacity_percent;

    let breach_count = (read_latency_breach as u8)
        .saturating_add(write_latency_breach as u8)
        .saturating_add(mount_failure_rate_breach as u8)
        .saturating_add(unmount_failure_rate_breach as u8)
        .saturating_add(path_validation_breach as u8)
        .saturating_add(mount_capacity_breach as u8);

    VfsMountSloReport {
        read_latency_p99_ticks: read_p99_ticks,
        write_latency_p99_ticks: write_p99_ticks,
        mount_failure_rate_per_mille: mount_failure_rate,
        unmount_failure_rate_per_mille: unmount_failure_rate,
        path_validation_failures: mount.path_validation_failures,
        total_mounts: mount.total_mounts,
        mount_capacity_percent,
        read_latency_breach,
        write_latency_breach,
        mount_failure_rate_breach,
        unmount_failure_rate_breach,
        path_validation_breach,
        mount_capacity_breach,
        breach_count,
    }
}

pub fn recommended_mount_health_action(report: VfsMountSloReport) -> VfsMountHealthAction {
    if report.mount_failure_rate_breach
        || report.unmount_failure_rate_breach
        || report.path_validation_breach
        || report.mount_capacity_breach
    {
        return VfsMountHealthAction::ThrottleMountChurn;
    }

    if report.read_latency_breach || report.write_latency_breach {
        return VfsMountHealthAction::PreferUnbufferedIo;
    }

    if !crate::config::KernelConfig::vfs_enable_buffered_io() {
        return VfsMountHealthAction::PreferBufferedIo;
    }

    VfsMountHealthAction::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn vfs_threshold_ticks_are_nonzero() {
        let t = mount_slo_thresholds();
        assert!(t.max_read_latency_ticks > 0);
        assert!(t.max_write_latency_ticks > 0);
    }

    #[test_case]
    fn vfs_thresholds_follow_runtime_overrides() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_vfs_health_max_mount_failure_rate_per_mille(Some(17));
        crate::config::KernelConfig::set_vfs_health_max_unmount_failure_rate_per_mille(Some(23));
        crate::config::KernelConfig::set_vfs_health_max_path_validation_failures(Some(31));
        crate::config::KernelConfig::set_vfs_health_max_mount_capacity_percent(Some(77));

        let t = mount_slo_thresholds();
        assert_eq!(t.max_mount_failure_rate_per_mille, 17);
        assert_eq!(t.max_unmount_failure_rate_per_mille, 23);
        assert_eq!(t.max_path_validation_failures, 31);
        assert_eq!(t.max_mount_capacity_percent, 77);

        crate::config::KernelConfig::reset_runtime_overrides();
    }

    #[test_case]
    fn action_prefers_unbuffered_on_latency_breach() {
        let report = VfsMountSloReport {
            read_latency_p99_ticks: 12,
            write_latency_p99_ticks: 9,
            mount_failure_rate_per_mille: 0,
            unmount_failure_rate_per_mille: 0,
            path_validation_failures: 0,
            total_mounts: 1,
            mount_capacity_percent: 1,
            read_latency_breach: true,
            write_latency_breach: false,
            mount_failure_rate_breach: false,
            unmount_failure_rate_breach: false,
            path_validation_breach: false,
            mount_capacity_breach: false,
            breach_count: 1,
        };
        assert_eq!(
            recommended_mount_health_action(report),
            VfsMountHealthAction::PreferUnbufferedIo
        );
    }

    #[test_case]
    fn action_prefers_throttle_for_mount_churn_breaches() {
        let report = VfsMountSloReport {
            read_latency_p99_ticks: 1,
            write_latency_p99_ticks: 1,
            mount_failure_rate_per_mille: 200,
            unmount_failure_rate_per_mille: 0,
            path_validation_failures: 0,
            total_mounts: 32,
            mount_capacity_percent: 95,
            read_latency_breach: false,
            write_latency_breach: false,
            mount_failure_rate_breach: true,
            unmount_failure_rate_breach: false,
            path_validation_breach: false,
            mount_capacity_breach: true,
            breach_count: 2,
        };
        assert_eq!(
            recommended_mount_health_action(report),
            VfsMountHealthAction::ThrottleMountChurn
        );
    }
}
