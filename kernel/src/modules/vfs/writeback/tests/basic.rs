use super::*;
use core::sync::atomic::Ordering;

#[test_case]
fn fsync_inode_flushes_dirty_cached_pages_and_updates_stats() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(7, sink.clone());
    register_inode(101, 7);

    let mut inode = cache::Inode::new(101, 0o100644);
    assert_eq!(inode.write_cached(0, b"hello writeback"), 15);
    let inode = Arc::new(inode);
    cache::GLOBAL_INODE_CACHE.insert(inode);

    let flushed = fsync_inode(101).expect("fsync should succeed");
    assert_eq!(flushed, 1);
    assert_eq!(sink.writes.lock().as_slice(), &[(101, 0, 4096)]);
    assert_eq!(sink.flushes.load(Ordering::Relaxed), 1);

    let stats = writeback_stats();
    assert_eq!(stats.total_flushes, 1);
    assert_eq!(stats.total_pages_written, 1);
    assert_eq!(stats.total_fsync_calls, 1);
    assert_eq!(stats.dirty_page_count, 0);

    cache::GLOBAL_INODE_CACHE.evict(101);
    reset_writeback_state_for_tests();
}

#[test_case]
fn unregister_sink_prunes_stale_dirty_keys_for_evicted_inode() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(9, sink);
    register_inode(202, 9);
    mark_dirty(202, 0);
    cache::GLOBAL_INODE_CACHE.evict(202);

    unregister_writable_mount(9).expect("unregister should flush stale state");

    let mgr = GLOBAL_WRITEBACK.lock();
    assert!(mgr.dirty_pages.is_empty());
    assert!(mgr.ino_to_mount.is_empty());
    assert!(!mgr.sinks.contains_key(&9));
    drop(mgr);

    reset_writeback_state_for_tests();
}

#[test_case]
fn fsync_inode_errors_when_inode_is_not_registered_to_a_mount() {
    reset_writeback_state_for_tests();

    let inode = Arc::new(cache::Inode::new(777, 0o100644));
    cache::GLOBAL_INODE_CACHE.insert(inode);

    assert_eq!(fsync_inode(777), Err("inode not associated with any mount"));

    cache::GLOBAL_INODE_CACHE.evict(777);
    reset_writeback_state_for_tests();
}

#[test_case]
fn fsync_inode_propagates_sink_write_failures() {
    reset_writeback_state_for_tests();

    register_writable_mount(51, Arc::new(FailingSink));
    register_inode(888, 51);

    let mut inode = cache::Inode::new(888, 0o100644);
    assert_eq!(inode.write_cached(0, b"boom"), 4);
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));

    assert_eq!(fsync_inode(888), Err("injected write failure"));

    cache::GLOBAL_INODE_CACHE.evict(888);
    reset_writeback_state_for_tests();
}
