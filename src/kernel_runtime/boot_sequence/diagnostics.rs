pub(crate) fn log_boot_diagnostics() {
    let diag = hypercore::kernel::startup::diagnostics();
    hypercore::klog_info!(
        "Boot complete: stages={} violations={} last={:?}",
        diag.transitions,
        diag.ordering_violations,
        diag.last_stage
    );
}
