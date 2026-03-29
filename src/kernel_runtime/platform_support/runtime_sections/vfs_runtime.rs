#[cfg(feature = "vfs")]
pub(crate) fn log_vfs_runtime_sections(
    telemetry: super::super::config::PlatformTelemetryConfig,
    verbose_inventory: bool,
) {
    if telemetry.vfs_runtime() {
        super::super::log_vfs_slo_thresholds();
        super::super::log_vfs_core_runtime();
    }

    if verbose_inventory {
        super::super::log_vfs_library_inventory();
    }
}
