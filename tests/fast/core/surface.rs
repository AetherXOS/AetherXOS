use hypercore::config::{BoundaryMode, KernelConfig};
use hypercore::modules::{
    is_library_surface_enabled, library_surface_policy, LibrarySurfacePolicy,
};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn library_surface_policy_tracks_boundary_mode() {
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
fn proc_surface_never_bypasses_vfs_gate() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_vfs_library_api_exposed(Some(false));
    KernelConfig::set_proc_config_api_exposed(Some(true));
    assert!(!is_library_surface_enabled("vfs"));
    assert!(!is_library_surface_enabled("proc_config"));

    KernelConfig::set_vfs_library_api_exposed(Some(true));
    assert!(is_library_surface_enabled("vfs"));
    assert!(is_library_surface_enabled("proc_config"));

    KernelConfig::reset_runtime_overrides();
}
