use super::*;
use alloc::vec;

fn commit_single_block(block: u64, fill: u8) -> u32 {
    let seq = begin_transaction(1).expect("journal transaction should start");
    assert!(journal_write(block, vec![fill; 8]));
    commit();
    seq
}

#[test_case]
fn commit_and_checkpoint_update_sequences_and_stats() {
    reset_journal_state_for_tests();
    init();

    let seq = begin_transaction(1).expect("journal should start");
    assert!(journal_write(42, vec![1, 2, 3, 4]));
    commit();

    let stats = super::stats();
    assert_eq!(stats.active_sequence, None);
    assert_eq!(stats.committed_sequence, seq);
    assert_eq!(stats.pending_tx_count, 1);
    assert_eq!(stats.total_commits, 1);
    assert_eq!(stats.blocks_journaled, 1);

    checkpoint();

    let stats = super::stats();
    assert_eq!(stats.checkpointed_sequence, seq);
    assert_eq!(stats.pending_tx_count, 0);
    assert_eq!(stats.total_checkpoints, 1);

    reset_journal_state_for_tests();
}

#[test_case]
fn revoke_suppresses_replay_of_older_committed_block() {
    reset_journal_state_for_tests();
    init();

    let seq1 = commit_single_block(7, 9);

    let seq2 = begin_transaction(1).expect("second transaction should start");
    assert!(seq2 > seq1);
    journal_revoke(7);
    commit();

    assert_eq!(recover(), 0);

    let stats = super::stats();
    assert_eq!(stats.total_commits, 2);
    assert_eq!(stats.total_revokes, 1);
    assert_eq!(stats.committed_sequence, seq2);

    reset_journal_state_for_tests();
}

#[test_case]
fn checkpoint_drains_pending_transactions_and_prevents_replay() {
    reset_journal_state_for_tests();
    init();

    assert!(begin_transaction(2).is_some());
    assert!(journal_write(9, vec![1; 32]));
    commit();
    checkpoint();

    let stats = super::stats();
    assert_eq!(stats.pending_tx_count, 0);
    assert_eq!(recover(), 0);

    reset_journal_state_for_tests();
}

#[test_case]
fn abort_blocks_new_transactions_and_writes() {
    reset_journal_state_for_tests();
    init();

    abort();

    assert_eq!(begin_transaction(1), None);
    assert!(!journal_write(12, vec![7; 8]));
    let stats = super::stats();
    assert!(stats.aborted);
    assert_eq!(stats.total_commits, 0);

    reset_journal_state_for_tests();
}

#[test_case]
fn begin_transaction_reuses_active_running_transaction() {
    reset_journal_state_for_tests();
    init();

    let first = begin_transaction(1).expect("first transaction");
    let second = begin_transaction(8).expect("same running transaction");
    assert_eq!(first, second);

    commit();
    reset_journal_state_for_tests();
}

#[test_case]
fn recover_replays_multiple_committed_transactions_in_sequence() {
    reset_journal_state_for_tests();
    init();

    let seq1 = commit_single_block(100, 1);
    let seq2 = begin_transaction(2).expect("tx2");
    assert!(seq2 > seq1);
    assert!(journal_write(200, vec![2; 8]));
    commit();

    assert_eq!(recover(), 2);

    let stats = stats();
    assert_eq!(stats.total_commits, 2);
    assert_eq!(stats.pending_tx_count, 2);

    reset_journal_state_for_tests();
}

#[test_case]
fn checkpoint_without_commits_keeps_sequences_unchanged() {
    reset_journal_state_for_tests();
    init();

    checkpoint();

    let stats = stats();
    assert_eq!(stats.committed_sequence, 0);
    assert_eq!(stats.checkpointed_sequence, 0);
    assert_eq!(stats.total_checkpoints, 0);

    reset_journal_state_for_tests();
}

#[test_case]
fn recovery_honors_revoke_only_for_older_or_equal_sequences() {
    reset_journal_state_for_tests();
    init();

    let seq1 = commit_single_block(55, 1);

    let seq2 = begin_transaction(2).expect("tx2");
    assert!(seq2 > seq1);
    assert!(journal_write(77, vec![2; 8]));
    journal_revoke(55);
    commit();

    let seq3 = begin_transaction(2).expect("tx3");
    assert!(seq3 > seq2);
    assert!(journal_write(55, vec![3; 8]));
    commit();

    assert_eq!(recover(), 2);

    reset_journal_state_for_tests();
}

#[test_case]
fn repeated_commit_checkpoint_cycles_behave_like_soak_smoke() {
    reset_journal_state_for_tests();
    init();

    for block in 0..16u64 {
        let _ = commit_single_block(block, block as u8);
        checkpoint();
    }

    let stats = stats();
    assert_eq!(stats.total_commits, 16);
    assert_eq!(stats.total_checkpoints, 16);
    assert_eq!(stats.pending_tx_count, 0);
    assert_eq!(stats.checkpointed_sequence, stats.committed_sequence);
    assert_eq!(recover(), 0);

    reset_journal_state_for_tests();
}

#[test_case]
fn recovery_keeps_sequence_order_across_interleaved_blocks() {
    reset_journal_state_for_tests();
    init();

    let seq1 = begin_transaction(3).expect("tx1");
    assert!(journal_write(10, vec![1; 8]));
    assert!(journal_write(20, vec![2; 8]));
    commit();

    let seq2 = begin_transaction(3).expect("tx2");
    assert!(seq2 > seq1);
    assert!(journal_write(30, vec![3; 8]));
    assert!(journal_write(40, vec![4; 8]));
    commit();

    assert_eq!(
        replayable_entries_for_tests(),
        vec![(10, seq1), (20, seq1), (30, seq2), (40, seq2)]
    );

    reset_journal_state_for_tests();
}

#[test_case]
fn checkpoint_prunes_revoke_entries_that_are_no_longer_needed() {
    reset_journal_state_for_tests();
    init();

    let seq1 = commit_single_block(90, 1);
    let seq2 = begin_transaction(2).expect("tx2");
    assert!(seq2 > seq1);
    journal_revoke(90);
    commit();
    assert_eq!(REVOKE_TABLE.lock().get(&90).copied(), Some(seq2));

    checkpoint();
    assert_eq!(REVOKE_TABLE.lock().get(&90).copied(), None);

    reset_journal_state_for_tests();
}

#[test_case]
fn repeated_replay_after_checkpoint_stays_empty() {
    reset_journal_state_for_tests();
    init();

    let _ = commit_single_block(101, 7);
    checkpoint();

    assert_eq!(recover(), 0);
    assert_eq!(recover(), 0);
    assert_eq!(replayable_entries_for_tests(), Vec::<(u64, u32)>::new());

    reset_journal_state_for_tests();
}

#[test_case]
fn abort_after_commit_preserves_existing_replay_plan_until_checkpoint() {
    reset_journal_state_for_tests();
    init();

    let seq = commit_single_block(303, 9);
    abort();

    assert_eq!(begin_transaction(1), None);
    assert!(!journal_write(404, vec![1; 4]));
    assert_eq!(replayable_entries_for_tests(), vec![(303, seq)]);
    assert_eq!(recover(), 1);

    checkpoint();
    assert_eq!(recover(), 0);

    reset_journal_state_for_tests();
}

#[test_case]
fn duplicate_revoke_entries_are_deduplicated_per_transaction() {
    reset_journal_state_for_tests();
    init();

    let seq = begin_transaction(2).expect("transaction");
    journal_revoke(404);
    journal_revoke(404);
    commit();

    assert_eq!(REVOKE_TABLE.lock().get(&404).copied(), Some(seq));
    assert_eq!(super::stats().total_revokes, 2);

    reset_journal_state_for_tests();
}

#[test_case]
fn checkpoint_prunes_only_revoke_entries_at_or_before_checkpointed_sequence() {
    reset_journal_state_for_tests();
    init();

    let seq1 = commit_single_block(1000, 1);
    let seq2 = begin_transaction(2).expect("tx2");
    assert!(seq2 > seq1);
    journal_revoke(1000);
    commit();

    let seq3 = begin_transaction(2).expect("tx3");
    assert!(seq3 > seq2);
    journal_revoke(2000);
    commit();

    checkpoint();

    let revoke = REVOKE_TABLE.lock();
    assert_eq!(revoke.get(&1000).copied(), None);
    assert_eq!(revoke.get(&2000).copied(), None);
    drop(revoke);

    reset_journal_state_for_tests();
}
