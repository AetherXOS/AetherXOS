
    use super::*;
    use crate::modules::vfs::cache;
    use alloc::sync::Arc;
    use alloc::vec;
    use spin::Mutex as SpinMutex;

    struct RecordingSink {
        writes: SpinMutex<Vec<(u64, u64, usize)>>,
        flushes: AtomicU64,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                writes: SpinMutex::new(Vec::new()),
                flushes: AtomicU64::new(0),
            }
        }
    }

    impl WritebackSink for RecordingSink {
        fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
            self.writes.lock().push((ino, offset, data.len()));
            Ok(())
        }

        fn flush(&self) -> Result<(), &'static str> {
            self.flushes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    struct FailingSink;

    impl WritebackSink for FailingSink {
        fn write_page(&self, _ino: u64, _offset: u64, _data: &[u8]) -> Result<(), &'static str> {
            Err("injected write failure")
        }

        fn flush(&self) -> Result<(), &'static str> {
            Ok(())
        }
    }

    struct TransactionalRecordingSink {
        writes: SpinMutex<Vec<(u64, u64, usize)>>,
        journal_entries: SpinMutex<Vec<u64>>,
        journal_commits: AtomicU64,
        flushes: AtomicU64,
    }

    impl TransactionalRecordingSink {
        fn new() -> Self {
            Self {
                writes: SpinMutex::new(Vec::new()),
                journal_entries: SpinMutex::new(Vec::new()),
                journal_commits: AtomicU64::new(0),
                flushes: AtomicU64::new(0),
            }
        }
    }

    fn reset_all_vfs_state_for_tests() {
        reset_writeback_state_for_tests();
        crate::modules::vfs::journal::reset_journal_state_for_tests();
    }

    fn assert_recovery_stays_empty_after_checkpoint(probes: usize) {
        crate::modules::vfs::journal::checkpoint();
        for _ in 0..probes {
            assert_eq!(crate::modules::vfs::journal::recover(), 0);
            crate::modules::vfs::journal::checkpoint();
        }
    }

    fn assert_recovery_survives_abort_until_checkpoint(expected: usize) {
        crate::modules::vfs::journal::abort();
        assert_eq!(crate::modules::vfs::journal::recover(), expected);
        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);
    }

    fn install_dirty_inode(ino: u64, mount_id: usize, payloads: &[&[u8]]) {
        register_inode(ino, mount_id);
        let mut inode = cache::Inode::new(ino, 0o100644);
        for (idx, payload) in payloads.iter().enumerate() {
            let offset = (idx * PAGE_SIZE) as u64;
            assert_eq!(inode.write_cached(offset, payload), payload.len());
        }
        cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));
    }

    impl WritebackSink for TransactionalRecordingSink {
        fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
            self.writes.lock().push((ino, offset, data.len()));
            Ok(())
        }

        fn flush(&self) -> Result<(), &'static str> {
            self.flushes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn journal_write(&self, entry: &JournalEntry) -> Result<(), &'static str> {
            self.journal_entries.lock().push(entry.seq);
            Ok(())
        }

        fn journal_commit(&self) -> Result<(), &'static str> {
            self.journal_commits.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

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
    fn periodic_writeback_flushes_oldest_dirty_page_first() {
        reset_writeback_state_for_tests();

        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(11, sink.clone());
        register_inode(303, 11);

        let inode = Arc::new(cache::Inode::new(303, 0o100644));
        {
            let mut pages = inode.pages.lock();
            let first = Arc::new(SpinMutex::new(cache::CachePage::new(0)));
            first.lock().dirty = true;
            let second = Arc::new(SpinMutex::new(cache::CachePage::new(4096)));
            second.lock().dirty = true;
            pages.insert(0, first);
            pages.insert(1, second);
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
            let mut pages = inode.pages.lock();
            let page = Arc::new(SpinMutex::new(cache::CachePage::new(0)));
            page.lock().dirty = true;
            pages.insert(0, page);
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

        crate::modules::vfs::cache::GLOBAL_INODE_CACHE.evict(1001);
        reset_writeback_state_for_tests();
    }

    #[test_case]
    fn journal_transaction_and_writeback_form_host_e2e_smoke_chain() {
        reset_all_vfs_state_for_tests();

        let sink = Arc::new(TransactionalRecordingSink::new());
        register_writable_mount(70, sink.clone());

        let mut txn = JournalTransaction::new();
        txn.add(JournalOp::InodeUpdate {
            ino: 2001,
            new_size: 4096,
            new_mode: 0o100644,
        });
        txn.add(JournalOp::DentryCreate {
            parent_ino: 1,
            name_hash: 0xAA55,
            child_ino: 2001,
        });
        txn.commit(sink.as_ref())
            .expect("journal transaction should commit");

        install_dirty_inode(2001, 70, &[b"journal+writeback"]);

        assert_eq!(fsync_inode(2001).expect("writeback should flush"), 1);
        assert_eq!(sink.journal_entries.lock().len(), 3);
        assert_eq!(sink.journal_commits.load(Ordering::Relaxed), 1);
        assert_eq!(sink.writes.lock().as_slice(), &[(2001, 0, 4096)]);
        assert!(sink.flushes.load(Ordering::Relaxed) >= 2);

        evict_inodes_for_tests(&[2001]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn journal_and_writeback_recovery_chain_preserves_replay_until_checkpoint() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(71, sink.clone());
        install_dirty_inode(3001, 71, &[b"page-a", b"page-b"]);

        assert!(crate::modules::vfs::journal::begin_transaction(2).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            3001,
            vec![1; 32]
        ));
        assert!(crate::modules::vfs::journal::journal_write(
            3002,
            vec![2; 32]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(
            crate::modules::vfs::journal::replayable_entries_for_tests(),
            vec![(3001, 1), (3002, 1)]
        );
        assert_eq!(fsync_inode(3001).expect("writeback flush"), 2);
        assert_eq!(crate::modules::vfs::journal::recover(), 2);

        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);

        evict_inodes_for_tests(&[3001]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn crash_recovery_soak_chain_handles_multiple_transactions_revoke_and_checkpoint() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(TransactionalRecordingSink::new());
        register_writable_mount(72, sink.clone());
        install_dirty_inode(4001, 72, &[b"a0", b"a1"]);
        install_dirty_inode(4002, 72, &[b"b0"]);

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            4001,
            vec![1; 16]
        ));
        assert!(crate::modules::vfs::journal::journal_write(
            5001,
            vec![2; 16]
        ));
        crate::modules::vfs::journal::commit();

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            4002,
            vec![3; 16]
        ));
        crate::modules::vfs::journal::journal_revoke(4001);
        crate::modules::vfs::journal::commit();

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            4001,
            vec![4; 16]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(
            crate::modules::vfs::journal::replayable_entries_for_tests(),
            vec![(5001, 1), (4002, 2), (4001, 3)]
        );

        assert_eq!(fsync_inode(4001).expect("inode 4001 flush"), 2);
        assert_eq!(fsync_inode(4002).expect("inode 4002 flush"), 1);
        assert_eq!(crate::modules::vfs::journal::recover(), 3);

        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);
        assert_eq!(sink.writes.lock().len(), 3);
        assert!(sink.flushes.load(Ordering::Relaxed) >= 2);

        evict_inodes_for_tests(&[4001, 4002]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn crash_recovery_chain_handles_multiple_mounts_without_cross_talk() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink_a = Arc::new(RecordingSink::new());
        let sink_b = Arc::new(RecordingSink::new());
        register_writable_mount(80, sink_a.clone());
        register_writable_mount(81, sink_b.clone());
        install_dirty_inode(5001, 80, &[b"mount-a-0", b"mount-a-1"]);
        install_dirty_inode(6001, 81, &[b"mount-b-0"]);

        assert!(crate::modules::vfs::journal::begin_transaction(4).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            5001,
            vec![1; 8]
        ));
        assert!(crate::modules::vfs::journal::journal_write(
            6001,
            vec![2; 8]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(
            crate::modules::vfs::journal::replayable_entries_for_tests(),
            vec![(5001, 1), (6001, 1)]
        );

        assert_eq!(fsync_inode(5001).expect("mount a flush"), 2);
        assert_eq!(fsync_inode(6001).expect("mount b flush"), 1);
        assert_eq!(sink_a.writes.lock().len(), 2);
        assert_eq!(sink_b.writes.lock().len(), 1);

        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);

        evict_inodes_for_tests(&[5001, 6001]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn crash_recovery_soak_chain_survives_interleaved_fsync_recover_and_checkpoint_cycles() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(TransactionalRecordingSink::new());
        register_writable_mount(90, sink.clone());
        install_dirty_inode(7001, 90, &[b"x0", b"x1"]);
        install_dirty_inode(7002, 90, &[b"y0"]);

        for cycle in 0..3u64 {
            assert!(crate::modules::vfs::journal::begin_transaction(4).is_some());
            assert!(crate::modules::vfs::journal::journal_write(
                7001 + cycle,
                vec![cycle as u8; 12]
            ));
            assert!(crate::modules::vfs::journal::journal_write(
                8001 + cycle,
                vec![cycle as u8 + 1; 12]
            ));
            if cycle == 1 {
                crate::modules::vfs::journal::journal_revoke(7001);
            }
            crate::modules::vfs::journal::commit();

            let _ = fsync_inode(7001);
            let _ = fsync_inode(7002);

            if cycle < 2 {
                assert!(crate::modules::vfs::journal::recover() >= 1);
            }
        }

        assert!(crate::modules::vfs::journal::recover() >= 4);
        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);
        assert!(sink.writes.lock().len() >= 3);
        assert!(sink.journal_entries.lock().len() >= 9);

        evict_inodes_for_tests(&[7001, 7002]);
        reset_all_vfs_state_for_tests();
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
        crate::modules::vfs::cache::GLOBAL_INODE_CACHE.insert(Arc::new(
            crate::modules::vfs::cache::Inode::new(8102, 0o100644),
        ));

        assert_eq!(sync_all().expect("sync_all should succeed"), 2);
        assert_eq!(sink_dirty.writes.lock().as_slice(), &[(8101, 0, 4096)]);
        assert!(sink_clean.writes.lock().is_empty());
        assert_eq!(sink_dirty.flushes.load(Ordering::Relaxed), 1);
        assert_eq!(sink_clean.flushes.load(Ordering::Relaxed), 1);

        evict_inodes_for_tests(&[8101, 8102]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn crash_recovery_chain_stays_empty_across_repeated_recover_and_checkpoint_calls() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        register_writable_mount(93, Arc::new(RecordingSink::new()));
        install_dirty_inode(8201, 93, &[b"once"]);

        assert!(crate::modules::vfs::journal::begin_transaction(2).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            8201,
            vec![1; 8]
        ));
        crate::modules::vfs::journal::commit();
        assert_eq!(fsync_inode(8201).expect("writeback should flush"), 1);
        assert_eq!(crate::modules::vfs::journal::recover(), 1);
        assert_recovery_stays_empty_after_checkpoint(2);

        evict_inodes_for_tests(&[8201]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn crash_recovery_soak_chain_remains_stable_after_multiple_post_checkpoint_probes() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(94, sink.clone());
        install_dirty_inode(8301, 94, &[b"alpha", b"beta"]);

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            8301,
            vec![3; 8]
        ));
        assert!(crate::modules::vfs::journal::journal_write(
            8302,
            vec![4; 8]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(fsync_inode(8301).expect("writeback should flush"), 2);
        assert_eq!(crate::modules::vfs::journal::recover(), 2);
        assert_recovery_stays_empty_after_checkpoint(4);

        assert_eq!(sink.writes.lock().len(), 2);

        evict_inodes_for_tests(&[8301]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn sync_all_after_recovery_commit_stays_quiet_after_checkpoint() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(95, sink.clone());
        install_dirty_inode(8401, 95, &[b"delta", b"gamma"]);

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            8401,
            vec![5; 8]
        ));
        assert!(crate::modules::vfs::journal::journal_write(
            8402,
            vec![6; 8]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(sync_all().expect("sync_all should succeed"), 1);
        assert_eq!(crate::modules::vfs::journal::recover(), 2);
        assert_recovery_stays_empty_after_checkpoint(3);
        assert_eq!(sink.writes.lock().len(), 2);
        assert!(sink.flushes.load(Ordering::Relaxed) >= 1);

        evict_inodes_for_tests(&[8401]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn abort_after_recovery_commit_preserves_existing_replay_until_checkpoint() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(96, sink.clone());
        install_dirty_inode(8501, 96, &[b"abort-me"]);

        assert!(crate::modules::vfs::journal::begin_transaction(2).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            8501,
            vec![7; 8]
        ));
        crate::modules::vfs::journal::commit();

        assert_eq!(fsync_inode(8501).expect("writeback should flush"), 1);
        assert_recovery_survives_abort_until_checkpoint(1);
        assert_eq!(sink.writes.lock().len(), 1);

        evict_inodes_for_tests(&[8501]);
        reset_all_vfs_state_for_tests();
    }

    #[test_case]
    fn sync_all_after_journal_abort_flushes_dirty_pages_without_accepting_new_replay_entries() {
        reset_all_vfs_state_for_tests();

        crate::modules::vfs::journal::init();
        let sink = Arc::new(RecordingSink::new());
        register_writable_mount(97, sink.clone());
        install_dirty_inode(8601, 97, &[b"before-abort", b"after-abort"]);

        assert!(crate::modules::vfs::journal::begin_transaction(3).is_some());
        assert!(crate::modules::vfs::journal::journal_write(
            8601,
            vec![8; 8]
        ));
        crate::modules::vfs::journal::commit();
        crate::modules::vfs::journal::abort();

        assert!(!crate::modules::vfs::journal::journal_write(
            8602,
            vec![9; 8]
        ));
        assert_eq!(sync_all().expect("sync_all should succeed"), 1);
        assert_eq!(crate::modules::vfs::journal::recover(), 1);
        crate::modules::vfs::journal::checkpoint();
        assert_eq!(crate::modules::vfs::journal::recover(), 0);
        assert_eq!(sink.writes.lock().len(), 2);

        evict_inodes_for_tests(&[8601]);
        reset_all_vfs_state_for_tests();
    }
