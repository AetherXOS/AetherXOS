use aethercore::config::{BoundaryMode, KernelConfig};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn linux_compat_blocker_matrix_stays_ordered_and_actionable() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Strict));
    KernelConfig::set_vfs_library_api_exposed(Some(false));
    KernelConfig::set_network_library_api_exposed(Some(false));
    KernelConfig::set_ipc_library_api_exposed(Some(false));

    let strict_codes = KernelConfig::linux_compat_blockers();
    assert!(strict_codes.contains(&"boundary_mode_strict_blocks_compat_surface"));
    assert!(strict_codes.contains(&"no_library_surface_exposed_for_compat"));
    assert_eq!(
        KernelConfig::linux_compat_next_action(),
        KernelConfig::linux_compat_blocker_details()[0].next_action
    );

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Balanced));
    KernelConfig::set_vfs_library_api_exposed(Some(true));

    let balanced_codes = KernelConfig::linux_compat_blockers();
    assert!(!balanced_codes.contains(&"boundary_mode_strict_blocks_compat_surface"));
    assert!(!balanced_codes.contains(&"no_library_surface_exposed_for_compat"));

    let details = KernelConfig::linux_compat_blocker_details();
    assert!(
        details
            .windows(2)
            .all(|pair| pair[0].severity <= pair[1].severity)
    );

    KernelConfig::reset_runtime_overrides();
}

#[test]
#[serial]
fn snapshot_profiles_preserve_cross_subsystem_override_consistency() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_telemetry_history_len(Some(32));
    KernelConfig::set_log_level_num(Some(5));
    KernelConfig::set_vfs_enable_buffered_io(Some(true));
    KernelConfig::set_diskfs_max_path_len(Some(128));
    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Balanced));
    KernelConfig::set_vfs_library_api_exposed(Some(true));
    KernelConfig::set_network_library_api_exposed(Some(false));
    KernelConfig::set_ipc_library_api_exposed(Some(true));

    let snapshot = KernelConfig::snapshot();

    assert_eq!(snapshot.core.vfs_max_mounts, KernelConfig::vfs_max_mounts());
    assert_eq!(
        snapshot.telemetry.history_len,
        KernelConfig::telemetry_history_len()
    );
    assert_eq!(
        snapshot.telemetry.log_level_num,
        KernelConfig::log_level_num()
    );
    assert_eq!(
        snapshot.vfs.enable_buffered_io,
        KernelConfig::vfs_enable_buffered_io()
    );
    assert_eq!(
        snapshot.vfs.diskfs_max_path_len,
        KernelConfig::diskfs_max_path_len()
    );
    assert_eq!(
        snapshot.library_runtime.boundary_mode,
        KernelConfig::boundary_mode()
    );
    assert_eq!(
        snapshot.library_runtime.expose_vfs_api,
        KernelConfig::is_vfs_library_api_exposed()
    );
    assert_eq!(
        snapshot.library_runtime.expose_network_api,
        KernelConfig::is_network_library_api_exposed()
    );
    assert_eq!(
        snapshot.library_runtime.expose_ipc_api,
        KernelConfig::is_ipc_library_api_exposed()
    );
    assert_eq!(
        snapshot.compat_surface.expose_linux_compat_surface,
        KernelConfig::should_expose_linux_compat_surface()
    );

    KernelConfig::reset_runtime_overrides();
}
