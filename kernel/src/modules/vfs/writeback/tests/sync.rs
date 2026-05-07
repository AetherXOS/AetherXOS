use super::*;
use core::sync::atomic::Ordering;

#[test_case]
fn sync_all_flushes_each_registered_mount() {
    reset_writeback_state_for_tests();

    let sink_a = Arc::new(RecordingSink::new());
    let sink_b = Arc::new(RecordingSink::new());
    register_writable_mount(21, sink_a.clone());
    register_writable_mount(22, sink_b.clone());
    register_inode(401, 21);
    register_inode(402, 22);

    let mut inode_a = cache::Inode::new(401, 0o100644);
    let mut inode_b = cache::Inode::new(402, 0o100644);
    assert_eq!(inode_a.write_cached(0, b"a"), 1);
    assert_eq!(inode_b.write_cached(0, b"b"), 1);
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode_a));
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode_b));

    assert_eq!(sync_all().expect("sync_all should succeed"), 2);
    assert_eq!(sink_a.writes.lock().len(), 1);
    assert_eq!(sink_b.writes.lock().len(), 1);
    assert_eq!(sink_a.flushes.load(Ordering::Relaxed), 1);
    assert_eq!(sink_b.flushes.load(Ordering::Relaxed), 1);

    evict_inodes_for_tests(&[401, 402]);
    reset_writeback_state_for_tests();
}

#[test_case]
fn sync_all_with_no_registered_mounts_is_a_noop() {
    reset_writeback_state_for_tests();

    assert_eq!(sync_all().expect("sync_all should succeed"), 0);
    let stats = writeback_stats();
    assert_eq!(stats.total_flushes, 0);
    assert_eq!(stats.total_pages_written, 0);

    reset_writeback_state_for_tests();
}

#[test_case]
fn unregister_sink_flushes_resident_dirty_pages_before_removal() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(31, sink.clone());
    register_inode(501, 31);

    let mut inode = cache::Inode::new(501, 0o100644);
    assert_eq!(inode.write_cached(0, b"persist me"), 10);
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));

    unregister_writable_mount(31).expect("unregister should flush resident dirty pages");

    assert_eq!(sink.writes.lock().as_slice(), &[(501, 0, 4096)]);
    assert_eq!(sink.flushes.load(Ordering::Relaxed), 1);

    cache::GLOBAL_INODE_CACHE.evict(501);
    reset_writeback_state_for_tests();
}

#[test_case]
fn sync_all_only_flushes_mounts_with_dirty_inodes_and_still_barriers_clean_mounts() {
    reset_all_vfs_state_for_tests();

    let sink_dirty = Arc::new(RecordingSink::new());
    let sink_clean = Arc::new(RecordingSink::new());
    register_writable_mount(91, sink_dirty.clone());
    register_writable_mount(92, sink_clean.clone());
    install_dirty_inode(8101, 91, &[b"dirty-page"]);
    register_inode(8102, 92);
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(
        cache::Inode::new(8102, 0o100644),
    ));

    assert_eq!(sync_all().expect("sync_all should succeed"), 2);
    assert_eq!(sink_dirty.writes.lock().as_slice(), &[(8101, 0, 4096)]);
    assert!(sink_clean.writes.lock().is_empty());
    assert_eq!(sink_dirty.flushes.load(Ordering::Relaxed), 1);
    assert_eq!(sink_clean.flushes.load(Ordering::Relaxed), 1);

    evict_inodes_for_tests(&[8101, 8102]);
    reset_all_vfs_state_for_tests();
}
