use aethercore::config::{BoundaryMode, KernelConfig};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn linux_compat_readiness_reflects_runtime_surface_inputs() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Strict));
    KernelConfig::set_vfs_library_api_exposed(Some(false));
    KernelConfig::set_network_library_api_exposed(Some(false));
    KernelConfig::set_ipc_library_api_exposed(Some(false));

    let strict = KernelConfig::linux_compat_readiness();
    assert!(!strict.boundary_allows_compat);
    assert!(!strict.vfs_api_exposed);
    assert!(!strict.network_api_exposed);
    assert!(!strict.ipc_api_exposed);
    assert!(!strict.effective_surface_enabled);

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Balanced));
    KernelConfig::set_network_library_api_exposed(Some(true));

    let balanced = KernelConfig::linux_compat_readiness();
    assert!(balanced.boundary_allows_compat);
    assert!(balanced.network_api_exposed);
    assert_eq!(
        balanced.effective_surface_enabled,
        balanced.compile_linux_compat
    );

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Compat));
    KernelConfig::set_vfs_library_api_exposed(Some(true));
    KernelConfig::set_ipc_library_api_exposed(Some(true));

    let compat = KernelConfig::linux_compat_readiness();
    assert!(compat.boundary_allows_compat);
    assert!(compat.vfs_api_exposed);
    assert!(compat.network_api_exposed);
    assert!(compat.ipc_api_exposed);
    assert_eq!(
        compat.effective_surface_enabled,
        compat.compile_linux_compat
    );

    KernelConfig::reset_runtime_overrides();
}

#[test]
#[serial]
fn config_snapshot_stays_aligned_with_runtime_overrides() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_telemetry_history_len(Some(12));
    KernelConfig::set_log_level_num(Some(4));
    KernelConfig::set_vfs_enable_buffered_io(Some(true));
    KernelConfig::set_diskfs_max_path_len(Some(96));
    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Compat));
    KernelConfig::set_vfs_library_api_exposed(Some(true));
    KernelConfig::set_network_library_api_exposed(Some(true));
    KernelConfig::set_ipc_library_api_exposed(Some(false));

    let snapshot = KernelConfig::snapshot();

    assert_eq!(snapshot.telemetry.history_len, 12);
    assert_eq!(snapshot.telemetry.log_level_num, 4);
    assert!(snapshot.vfs.enable_buffered_io);
    assert_eq!(snapshot.vfs.diskfs_max_path_len, 96);
    assert_eq!(snapshot.library_runtime.boundary_mode, BoundaryMode::Compat);
    assert!(snapshot.library_runtime.expose_vfs_api);
    assert!(snapshot.library_runtime.expose_network_api);
    assert!(!snapshot.library_runtime.expose_ipc_api);
    assert_eq!(
        snapshot.compat_surface.expose_linux_compat_surface,
        KernelConfig::should_expose_linux_compat_surface()
    );

    KernelConfig::reset_runtime_overrides();
}
