use super::*;

#[test_case]
fn periodic_writeback_flushes_oldest_dirty_page_first() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(11, sink.clone());
    register_inode(303, 11);

    let inode = Arc::new(cache::Inode::new(303, 0o100644));
    {
        let first = Arc::new(SpinMutex::new(cache::CachePage::new(0)));
        first.lock().dirty = true;
        let second = Arc::new(SpinMutex::new(cache::CachePage::new(4096)));
        second.lock().dirty = true;
        let first_shard = inode.get_page_shard(0);
        inode.pages[first_shard].lock().insert(0, first);
        let second_shard = inode.get_page_shard(1);
        inode.pages[second_shard].lock().insert(1, second);
    }
    cache::GLOBAL_INODE_CACHE.insert(inode.clone());

    {
        let mut mgr = GLOBAL_WRITEBACK.lock();
        mgr.dirty_pages.insert(
            DirtyPageKey {
                ino: 303,
                page_idx: 0,
            },
            DirtyPageEntry {
                dirty_since: 10,
                redirty_count: 1,
            },
        );
        mgr.dirty_pages.insert(
            DirtyPageKey {
                ino: 303,
                page_idx: 1,
            },
            DirtyPageEntry {
                dirty_since: 20,
                redirty_count: 1,
            },
        );
        assert_eq!(mgr.flush_oldest(1).expect("flush oldest should work"), 1);
        assert!(mgr.dirty_pages.contains_key(&DirtyPageKey {
            ino: 303,
            page_idx: 1
        }));
        assert!(!mgr.dirty_pages.contains_key(&DirtyPageKey {
            ino: 303,
            page_idx: 0
        }));
    }

    assert_eq!(sink.writes.lock().as_slice(), &[(303, 0, 4096)]);
    cache::GLOBAL_INODE_CACHE.evict(303);
    reset_writeback_state_for_tests();
}

#[test_case]
fn periodic_writeback_advances_tick_and_updates_stats() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(41, sink.clone());
    register_inode(601, 41);

    let mut inode = cache::Inode::new(601, 0o100644);
    assert_eq!(inode.write_cached(0, b"tick"), 4);
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));

    assert_eq!(periodic_writeback(77), 1);

    let mgr = GLOBAL_WRITEBACK.lock();
    assert_eq!(mgr.current_tick, 77);
    drop(mgr);

    let stats = writeback_stats();
    assert_eq!(stats.total_flushes, 1);
    assert_eq!(stats.total_pages_written, 1);
    assert_eq!(stats.dirty_page_count, 0);

    evict_inodes_for_tests(&[601]);
    reset_writeback_state_for_tests();
}

#[test_case]
fn periodic_writeback_with_no_dirty_pages_keeps_stats_stable() {
    reset_writeback_state_for_tests();

    assert_eq!(periodic_writeback(123), 0);
    let mgr = GLOBAL_WRITEBACK.lock();
    assert_eq!(mgr.current_tick, 123);
    drop(mgr);

    let stats = writeback_stats();
    assert_eq!(stats.total_flushes, 0);
    assert_eq!(stats.total_pages_written, 0);
    assert_eq!(stats.dirty_page_count, 0);

    reset_writeback_state_for_tests();
}

#[test_case]
fn repeated_periodic_writeback_cycles_drain_multiple_inodes_like_soak_smoke() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(61, sink.clone());

    for ino in 900..904u64 {
        register_inode(ino, 61);
        let mut inode = cache::Inode::new(ino, 0o100644);
        assert_eq!(inode.write_cached(0, b"page-0"), 6);
        assert_eq!(inode.write_cached(4096, b"page-1"), 6);
        cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));
    }

    let mut total = 0usize;
    for tick in 1..=8u64 {
        total += periodic_writeback(tick);
    }

    assert_eq!(total, 8);
    assert_eq!(sink.writes.lock().len(), 8);
    let stats = writeback_stats();
    assert_eq!(stats.dirty_page_count, 0);
    assert_eq!(stats.total_pages_written, 8);

    evict_inodes_for_tests(&[900, 901, 902, 903]);
    reset_writeback_state_for_tests();
}

#[test_case]
fn periodic_writeback_skips_unregistered_inodes_and_cleans_keys() {
    reset_writeback_state_for_tests();

    let sink = Arc::new(RecordingSink::new());
    register_writable_mount(62, sink.clone());

    let inode = Arc::new(cache::Inode::new(1001, 0o100644));
    {
        let page = Arc::new(SpinMutex::new(cache::CachePage::new(0)));
        page.lock().dirty = true;
        let shard = inode.get_page_shard(0);
        inode.pages[shard].lock().insert(0, page);
    }
    cache::GLOBAL_INODE_CACHE.insert(inode);

    {
        let mut mgr = GLOBAL_WRITEBACK.lock();
        mgr.dirty_pages.insert(
            DirtyPageKey {
                ino: 1001,
                page_idx: 0,
            },
            DirtyPageEntry {
                dirty_since: 1,
                redirty_count: 1,
            },
        );
    }

    assert_eq!(periodic_writeback(5), 0);
    let mgr = GLOBAL_WRITEBACK.lock();
    assert!(mgr.dirty_pages.is_empty());
    drop(mgr);
    assert!(sink.writes.lock().is_empty());

    cache::GLOBAL_INODE_CACHE.evict(1001);
    reset_writeback_state_for_tests();
}
