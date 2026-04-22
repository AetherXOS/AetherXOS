use super::*;

#[test_case]
fn mount_access_policy_allows_owner_and_root_only() {
    let entry = MountEntry {
        id: 1,
        fs_kind: MountFsKind::RamFs,
        path: b"/tenant-a".to_vec(),
        path_len: 9,
        owner: TaskId(7),
        readonly: false,
    };

    assert!(can_access_mount(entry.owner, ROOT_TASK_ID));
    assert!(can_access_mount(entry.owner, TaskId(7)));
    assert!(!can_access_mount(entry.owner, TaskId(8)));
}

#[test_case]
fn normalize_mount_path_rejects_parent_traversal_and_nuls() {
    let max = crate::config::KernelConfig::vfs_max_mount_path();
    assert_eq!(
        normalize_mount_path(b"/tenant//logs/", max),
        Some(b"/tenant/logs".to_vec())
    );
    assert!(normalize_mount_path(b"/tenant/../escape", max).is_none());
    assert!(normalize_mount_path(b"/tenant/\0", max).is_none());
}

#[test_case]
fn mount_relocate_and_readonly_flow_stays_consistent() {
    let mount_id = mount_ramfs(b"/policy-a").expect("mount");
    assert_eq!(mount_id_by_path(b"/policy-a"), Some(mount_id));
    assert_eq!(mount_readonly_by_id(mount_id), Some(false));

    set_mount_readonly(mount_id, true).expect("readonly");
    assert_eq!(mount_readonly_by_path(b"/policy-a"), Some(true));

    relocate_mount(mount_id, b"/policy-b").expect("relocate");
    assert_eq!(mount_id_by_path(b"/policy-a"), None);
    assert_eq!(mount_id_by_path(b"/policy-b"), Some(mount_id));
    assert_eq!(mount_readonly_by_path(b"/policy-b"), Some(true));

    unmount(mount_id).expect("unmount");
}

#[test_case]
fn list_mounts_and_path_lookup_roundtrip() {
    let mount_id = mount_ramfs(b"/roundtrip").expect("mount");
    let mut records = [MountRecord {
        id: 0,
        fs_kind: 0,
        path_len: 0,
    }; 4];
    let written = list_mounts(&mut records);
    assert!(written >= 1);
    assert!(records[..written]
        .iter()
        .any(|record| { record.id == mount_id && record.fs_kind == MountFsKind::RamFs as usize }));

    let mut out = [0u8; 32];
    let len = mount_path_by_id(mount_id, &mut out).expect("path");
    assert_eq!(&out[..len], b"/roundtrip");

    unmount_by_path(b"/roundtrip").expect("unmount by path");
}

#[test_case]
fn initrd_loader_rejects_invalid_paths_and_tracks_failures() {
    let mount_id = mount_ramfs(b"/initrd-check").expect("mount");
    let before = stats();
    let err = load_initrd_entries(mount_id, &[("../escape", b"x")]).unwrap_err();
    assert_eq!(err, "invalid initrd path");
    let after = stats();
    assert_eq!(after.initrd_load_failures, before.initrd_load_failures + 1);
    unmount(mount_id).expect("unmount");
}

#[test_case]
fn mount_registry_differential_behavior_tracks_core_operations() {
    let before = stats();

    let mount_a = mount_ramfs(b"/matrix-a").expect("mount a");
    assert_eq!(mount_id_by_path(b"/matrix-a"), Some(mount_a));
    assert!(mount_ramfs(b"/matrix-a").is_err());

    let mount_b = mount_ramfs(b"/matrix-b").expect("mount b");
    relocate_mount(mount_b, b"/matrix-c").expect("relocate b");
    assert_eq!(mount_id_by_path(b"/matrix-b"), None);
    assert_eq!(mount_id_by_path(b"/matrix-c"), Some(mount_b));

    load_initrd_entries(
        mount_a,
        &[("/matrix-a/init.txt", b"alpha"), ("/matrix-a/trace.txt", b"beta")],
    )
    .expect("load initrd entries");

    unmount_by_path(b"/matrix-c").expect("unmount relocated mount");
    unmount(mount_a).expect("unmount mount a");

    let after = stats();
    assert!(after.mount_attempts >= before.mount_attempts + 3);
    assert!(after.mount_success >= before.mount_success + 2);
    assert!(after.mount_failures >= before.mount_failures + 1);
    assert!(after.unmount_success >= before.unmount_success + 2);
    assert!(after.unmount_by_path_success >= before.unmount_by_path_success + 1);
    assert!(after.initrd_load_calls >= before.initrd_load_calls + 1);
    assert!(after.initrd_load_files >= before.initrd_load_files + 2);
    assert!(after.initrd_load_bytes >= before.initrd_load_bytes + 9);
}
