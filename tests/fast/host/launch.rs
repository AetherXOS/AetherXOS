use aethercore::kernel::launch::{LaunchError, spawn_bootstrap_from_image};
use aethercore::kernel::module_loader::{
    ModuleLoadError, build_load_plan, inspect_elf_image, snapshot_module_image,
};

use crate::common::elf::minimal_loadable_image;

#[test]
fn module_snapshot_matches_public_loader_views() {
    let probe = minimal_loadable_image();
    let snapshot = snapshot_module_image(&probe).expect("snapshot");
    let info = inspect_elf_image(&probe).expect("inspect");
    let plan = build_load_plan(&probe).expect("plan");

    assert_eq!(snapshot.info.entry, info.entry);
    assert_eq!(snapshot.info.machine, info.machine);
    assert_eq!(snapshot.load_plan.segments.len(), plan.segments.len());
    assert_eq!(snapshot.load_plan.total_file_bytes, plan.total_file_bytes);
    assert_eq!(snapshot.load_plan.total_mem_bytes, plan.total_mem_bytes);
}

#[test]
fn loader_rejects_invalid_requests_without_bootstrap_side_effects() {
    assert!(matches!(
        inspect_elf_image(&[0u8; 8]),
        Err(ModuleLoadError::TooSmall)
    ));
    assert_eq!(
        spawn_bootstrap_from_image(b"", &[1u8], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        spawn_bootstrap_from_image(b"probe", &[], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}
