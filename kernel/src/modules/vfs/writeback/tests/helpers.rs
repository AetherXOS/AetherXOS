use super::*;

pub fn reset_all_vfs_state_for_tests() {
    reset_writeback_state_for_tests();
    crate::modules::vfs::journal::reset_journal_state_for_tests();
}

pub fn assert_recovery_stays_empty_after_checkpoint(probes: usize) {
    crate::modules::vfs::journal::checkpoint();
    for _ in 0..probes {
        assert_eq!(crate::modules::vfs::journal::recover(), 0);
        crate::modules::vfs::journal::checkpoint();
    }
}

pub fn assert_recovery_survives_abort_until_checkpoint(expected: usize) {
    crate::modules::vfs::journal::abort();
    assert_eq!(crate::modules::vfs::journal::recover(), expected);
    crate::modules::vfs::journal::checkpoint();
    assert_eq!(crate::modules::vfs::journal::recover(), 0);
}

pub fn install_dirty_inode(ino: u64, mount_id: usize, payloads: &[&[u8]]) {
    register_inode(ino, mount_id);
    let mut inode = cache::Inode::new(ino, 0o100644);
    for (idx, payload) in payloads.iter().enumerate() {
        let offset = (idx * PAGE_SIZE) as u64;
        assert_eq!(inode.write_cached(offset, payload), payload.len());
    }
    cache::GLOBAL_INODE_CACHE.insert(Arc::new(inode));
}

