use alloc::collections::BTreeMap;

use super::journal::JournalEntry;

pub(super) fn should_replay_entry(revoke: &BTreeMap<u64, u32>, entry: &JournalEntry) -> bool {
    match revoke.get(&entry.fs_block) {
        Some(&rev_seq) => rev_seq < entry.sequence,
        None => true,
    }
}

pub(super) fn prune_revoke_table(revoke: &mut BTreeMap<u64, u32>, checkpointed_seq: u32) {
    revoke.retain(|_, sequence| *sequence > checkpointed_seq);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn test_entry(block: u64, sequence: u32) -> JournalEntry {
        JournalEntry {
            fs_block: block,
            sequence,
            data: vec![sequence as u8; 4],
            dirty: true,
        }
    }

    #[test_case]
    fn replay_helper_replays_entries_without_revoke_or_with_older_revoke() {
        let entry = test_entry(11, 3);
        let mut revoke = BTreeMap::new();
        assert!(should_replay_entry(&revoke, &entry));
        revoke.insert(11, 2);
        assert!(should_replay_entry(&revoke, &entry));
    }

    #[test_case]
    fn replay_helper_blocks_entries_with_equal_or_newer_revoke_sequence() {
        let entry = test_entry(11, 3);
        let mut revoke = BTreeMap::new();
        revoke.insert(11, 3);
        assert!(!should_replay_entry(&revoke, &entry));
        revoke.insert(11, 4);
        assert!(!should_replay_entry(&revoke, &entry));
    }

    #[test_case]
    fn revoke_pruner_keeps_only_entries_newer_than_checkpoint() {
        let mut revoke = BTreeMap::new();
        revoke.insert(10, 2);
        revoke.insert(20, 5);
        revoke.insert(30, 8);
        prune_revoke_table(&mut revoke, 5);
        assert_eq!(revoke.get(&10), None);
        assert_eq!(revoke.get(&20), None);
        assert_eq!(revoke.get(&30).copied(), Some(8));
    }
}
