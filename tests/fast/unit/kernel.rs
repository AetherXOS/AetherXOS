use aethercore::kernel::launch::{LaunchError, spawn_bootstrap_from_image, stats};
use aethercore::kernel::module_loader::ModuleLoadError;
use aethercore::kernel::module_loader::inspect_elf_image;

#[test]
fn launch_stats_snapshot_remains_self_consistent() {
    let snapshot = stats();

    assert!(snapshot.spawn_attempts >= snapshot.spawn_success);
    assert!(snapshot.spawn_attempts >= snapshot.spawn_failures);
    assert!(snapshot.claim_attempts >= snapshot.claim_success);
    assert!(snapshot.claim_attempts >= snapshot.claim_failures);
    assert!(snapshot.handoff_ack_attempts >= snapshot.handoff_ack_success);
    assert!(snapshot.handoff_ack_attempts >= snapshot.handoff_ack_failures);
    assert!(snapshot.handoff_consume_attempts >= snapshot.handoff_consume_success);
    assert!(snapshot.handoff_consume_attempts >= snapshot.handoff_consume_failures);
}

#[test]
fn invalid_bootstrap_requests_are_rejected_before_launch() {
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
