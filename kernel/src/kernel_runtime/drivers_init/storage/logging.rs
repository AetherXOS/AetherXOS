pub(super) fn log_storage_probe_report(
    probe_report: &hypercore::modules::drivers::StorageProbeReport,
) {
    hypercore::klog_info!(
        "Storage probe report: steps={} found={} init_ok={} init_fail={}",
        probe_report.probe_steps,
        probe_report.probed_drivers,
        probe_report.init_success,
        probe_report.init_failures
    );
}

pub(super) fn log_storage_driver_stats(
    device_count: usize,
    block_stats: &hypercore::modules::drivers::block::BlockDriverStats,
) {
    hypercore::klog_info!(
        "Storage drivers: devices={} probe={}/{} init={}/{} io={}/{}",
        device_count,
        block_stats.probe_success,
        block_stats.probe_attempts,
        block_stats.init_success,
        block_stats.init_attempts,
        block_stats.io_success,
        block_stats.io_attempts
    );
}

pub(super) fn log_storage_lifecycle(
    lifecycle: &hypercore::modules::drivers::StorageLifecycleSummary,
) {
    hypercore::klog_info!(
        "Storage lifecycle: total={} healthy={} degraded={} failed={}",
        lifecycle.total,
        lifecycle.healthy,
        lifecycle.degraded,
        lifecycle.failed
    );
}

pub(super) fn log_driver_wait_policy() {
    let waits = hypercore::modules::drivers::wait_policy_snapshot();
    hypercore::klog_info!(
        "Driver wait policy: {} {} {} {} {} {} {}",
        wait_policy_segment(&waits.nvme_disable_ready),
        wait_policy_segment(&waits.nvme_controller_ready),
        wait_policy_segment(&waits.nvme_admin),
        wait_policy_segment(&waits.nvme_io),
        wait_policy_segment(&waits.ahci_read),
        wait_policy_segment(&waits.ahci_write),
        wait_policy_segment(&waits.e1000_reset)
    );
}

fn wait_policy_segment(
    wait: &hypercore::modules::drivers::DriverWaitPolicySnapshotEntry,
) -> String {
    format!(
        "{}::{} max_spins={} fallback={:?} timeouts={}",
        wait.driver, wait.operation, wait.max_spins, wait.fallback, wait.timeout_events
    )
}
