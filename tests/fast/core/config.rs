use hypercore::config::{BoundaryMode, KernelConfig};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn telemetry_history_override_stays_bounded() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_telemetry_history_len(Some(usize::MAX));
    assert_eq!(KernelConfig::telemetry_history_len(), 1_000_000);

    KernelConfig::set_telemetry_history_len(Some(8));
    assert_eq!(KernelConfig::telemetry_history_len(), 8);

    KernelConfig::reset_runtime_overrides();
    assert!(KernelConfig::telemetry_history_len() >= 1);
}

#[test]
#[serial]
fn boundary_mode_preserves_explicit_proc_and_sysctl_toggles() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_vfs_library_api_exposed(Some(true));
    KernelConfig::set_proc_config_api_exposed(Some(true));
    KernelConfig::set_sysctl_api_exposed(Some(true));

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Strict));
    assert!(KernelConfig::should_expose_procfs_surface());
    assert!(KernelConfig::should_expose_sysctl_surface());
    assert!(!KernelConfig::should_expose_linux_compat_surface());

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Compat));
    assert!(KernelConfig::should_expose_procfs_surface());
    assert!(KernelConfig::should_expose_sysctl_surface());

    KernelConfig::reset_runtime_overrides();
}
