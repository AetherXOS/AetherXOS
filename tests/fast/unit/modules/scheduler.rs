use aethercore::config::{BoundaryMode, KernelConfig};
use aethercore::modules::{
    LibrarySurfacePolicy, is_library_surface_enabled, library_surface_policy,
};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn library_surface_policy_tracks_boundary_modes() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Strict));
    assert_eq!(library_surface_policy(), LibrarySurfacePolicy::CoreOnly);

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Balanced));
    assert_eq!(
        library_surface_policy(),
        LibrarySurfacePolicy::CorePlusSelectedLibraries
    );

    KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Compat));
    assert_eq!(library_surface_policy(), LibrarySurfacePolicy::CompatAll);

    KernelConfig::reset_runtime_overrides();
}

#[test]
#[serial]
fn library_surface_gate_matrix_stays_explicit() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_vfs_library_api_exposed(Some(false));
    KernelConfig::set_network_library_api_exposed(Some(true));
    KernelConfig::set_ipc_library_api_exposed(Some(true));
    KernelConfig::set_proc_config_api_exposed(Some(true));
    KernelConfig::set_sysctl_api_exposed(Some(true));

    assert!(!is_library_surface_enabled("vfs"));
    assert!(is_library_surface_enabled("network"));
    assert!(is_library_surface_enabled("ipc"));
    assert!(!is_library_surface_enabled("proc_config"));
    assert!(!is_library_surface_enabled("sysctl"));
    assert!(!is_library_surface_enabled("does_not_exist"));

    KernelConfig::set_vfs_library_api_exposed(Some(true));
    assert!(is_library_surface_enabled("proc_config"));
    assert!(is_library_surface_enabled("sysctl"));

    KernelConfig::reset_runtime_overrides();
}
