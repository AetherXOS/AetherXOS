use super::*;

#[test_case]
fn mount_devfs_exposes_builtin_nodes() {
    let fs_id = mount_devfs("/dev-test").expect("mount devfs");
    assert!(access(fs_id, "/null").expect("access null"));
    assert!(access(fs_id, "/zero").expect("access zero"));
    assert!(access(fs_id, "/random").expect("access random"));
    assert!(access(fs_id, "/urandom").expect("access urandom"));
    let _ = unmount(fs_id);
}

#[test_case]
fn devfs_event_api_returns_events() {
    let fs_id = mount_devfs("/dev-test-events").expect("mount devfs");
    let snapshot = devfs_event_snapshot(fs_id).expect("snapshot");
    assert!(snapshot.queued > 0);

    let events = devfs_events_since(fs_id, 0, 64).expect("events");
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| e.path == "null"));
    let _ = unmount(fs_id);
}