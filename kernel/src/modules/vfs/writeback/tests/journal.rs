use super::*;
use core::sync::atomic::Ordering;
use super::super::{JournalTransaction, JournalOp};

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
