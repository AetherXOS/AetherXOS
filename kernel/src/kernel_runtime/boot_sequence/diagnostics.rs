pub(crate) fn log_boot_diagnostics() {
    let diag = aethercore::kernel::startup::diagnostics();
    aethercore::klog_info!(
        "Boot complete: stages={} violations={} last={:?}",
        diag.transitions,
        diag.ordering_violations,
        diag.last_stage
    );

    #[cfg(feature = "linux_userspace_graphics")]
    {
        aethercore::modules::userspace_graphics::log_stack_summary("boot_sequence");
    }
}
