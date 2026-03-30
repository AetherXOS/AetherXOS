use hypercore::kernel::launch::{spawn_bootstrap_from_image, LaunchError};
use hypercore::kernel::module_loader::{
    build_load_plan, inspect_elf_image, snapshot_module_image, ModuleLoadError,
};

static PROBE: &[u8] = include_bytes!("../../../boot/initramfs/usr/lib/hypercore/probe-linked.elf");

#[test]
fn module_snapshot_matches_public_loader_views() {
    let snapshot = snapshot_module_image(PROBE).expect("snapshot");
    let info = inspect_elf_image(PROBE).expect("inspect");
    let plan = build_load_plan(PROBE).expect("plan");

    assert_eq!(snapshot.info.entry, info.entry);
    assert_eq!(snapshot.info.machine, info.machine);
    assert_eq!(snapshot.load_plan.segments.len(), plan.segments.len());
    assert_eq!(snapshot.load_plan.total_file_bytes, plan.total_file_bytes);
    assert_eq!(snapshot.load_plan.total_mem_bytes, plan.total_mem_bytes);
}

#[test]
fn loader_rejects_invalid_requests_without_bootstrap_side_effects() {
    assert_eq!(inspect_elf_image(&[0u8; 8]), Err(ModuleLoadError::TooSmall));
    assert_eq!(
        spawn_bootstrap_from_image(b"", &[1u8], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        spawn_bootstrap_from_image(b"probe", &[], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}
