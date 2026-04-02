pub(crate) fn log_boot_diagnostics() {
    let diag = aethercore::kernel::startup::diagnostics();
    aethercore::klog_info!(
        "Boot complete: stages={} violations={} last={:?}",
        diag.transitions,
        diag.ordering_violations,
        diag.last_stage
    );
}
