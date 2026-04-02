use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};

use aethercore::modules::vfs::{
    self,
    journal,
    writeback::{self, WritebackSink},
    BlockDeviceAdapter,
    BlockWritebackSink,
    Inode,
    JournalEntry,
    GLOBAL_INODE_CACHE,
};

fn vfs_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_vfs() -> std::sync::MutexGuard<'static, ()> {
    vfs_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct RecordingSink {
    writes: Mutex<Vec<(u64, u64, usize)>>,
    flushes: AtomicU64,
    journal_entries: Mutex<Vec<u64>>,
    journal_commits: AtomicU64,
}

impl RecordingSink {
    fn new() -> Self {
        Self {
            writes: Mutex::new(Vec::new()),
            flushes: AtomicU64::new(0),
            journal_entries: Mutex::new(Vec::new()),
            journal_commits: AtomicU64::new(0),
        }
    }
}

impl WritebackSink for RecordingSink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        self.writes.lock().expect("writes").push((ino, offset, data.len()));
        Ok(())
    }

    fn flush(&self) -> Result<(), &'static str> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn journal_write(&self, entry: &JournalEntry) -> Result<(), &'static str> {
        self.journal_entries
            .lock()
            .expect("journal entries")
            .push(entry.seq);
        Ok(())
    }

    fn journal_commit(&self) -> Result<(), &'static str> {
        self.journal_commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Clone)]
struct SharedBlockDevice {
    blocks: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl SharedBlockDevice {
    fn new(block_count: usize) -> Self {
        Self {
            blocks: Arc::new(Mutex::new(vec![vec![0; 4096]; block_count])),
        }
    }

    fn snapshot_block(&self, block: usize) -> Vec<u8> {
        self.blocks.lock().expect("blocks")[block].clone()
    }
}

fn journal_seq(block: &[u8]) -> u64 {
    let mut seq = [0u8; 8];
    seq.copy_from_slice(&block[..8]);
    u64::from_le_bytes(seq)
}

fn journal_op(block: &[u8]) -> u32 {
    let mut op = [0u8; 4];
    op.copy_from_slice(&block[8..12]);
    u32::from_le_bytes(op)
}

fn occupied_journal_slots(
    device: &SharedBlockDevice,
    journal_blocks: u64,
) -> Vec<(usize, u64, u32)> {
    (0..journal_blocks as usize)
        .filter_map(|slot| {
            let block = device.snapshot_block(slot);
            let seq = journal_seq(&block);
            if seq == 0 {
                None
            } else {
                Some((slot, seq, journal_op(&block)))
            }
        })
        .collect()
}

fn mount_disk_sink(
    mount_id: usize,
    backing: &SharedBlockDevice,
    journal_blocks: u64,
) -> Arc<BlockWritebackSink> {
    let sink = Arc::new(BlockWritebackSink::new(
        Box::new(backing.clone()),
        journal_blocks,
    ));
    writeback::register_writable_mount(mount_id, sink.clone());
    sink
}

fn install_cached_inode(ino: u64, pages: &[&[u8]]) {
    let mut inode = Inode::new(ino, 0o100644);
    for (idx, page) in pages.iter().enumerate() {
        assert_eq!(inode.write_cached((idx * 4096) as u64, page), page.len());
    }
    GLOBAL_INODE_CACHE.insert(Arc::new(inode));
}

fn assert_block_prefix(device: &SharedBlockDevice, block: usize, expected: &[u8]) {
    let snapshot = device.snapshot_block(block);
    assert!(snapshot.starts_with(expected));
}

impl BlockDeviceAdapter for SharedBlockDevice {
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        let blocks = self.blocks.lock().map_err(|_| "poisoned block device")?;
        let src = blocks.get(block as usize).ok_or("block out of range")?;
        if buf.len() > src.len() {
            return Err("buffer larger than block");
        }
        buf.copy_from_slice(&src[..buf.len()]);
        Ok(())
    }

    fn write_block(&mut self, block: u64, data: &[u8]) -> Result<(), &'static str> {
        let mut blocks = self.blocks.lock().map_err(|_| "poisoned block device")?;
        let dst = blocks.get_mut(block as usize).ok_or("block out of range")?;
        if data.len() > dst.len() {
            return Err("write larger than block");
        }
        dst[..data.len()].copy_from_slice(data);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn block_count(&self) -> u64 {
        self.blocks.lock().expect("blocks").len() as u64
    }
}

#[test]
fn replay_survives_abort_until_checkpoint_on_host() {
    let _guard = lock_vfs();

    journal::init();
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(1001, vec![1; 8]));
    journal::commit();

    journal::abort();

    assert_eq!(journal::recover(), 1);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);
}

#[test]
fn revoke_blocks_older_entry_but_not_newer_one_on_host() {
    let _guard = lock_vfs();

    journal::init();

    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(2001, vec![1; 8]));
    journal::commit();

    assert!(journal::begin_transaction(2).is_some());
    journal::journal_revoke(2001);
    assert!(journal::journal_write(3001, vec![2; 8]));
    journal::commit();

    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(2001, vec![3; 8]));
    journal::commit();

    assert_eq!(journal::recover(), 2);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);
}

#[test]
fn repeated_checkpoint_and_recover_stay_stable_on_host() {
    let _guard = lock_vfs();

    journal::init();
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(4001, vec![4; 8]));
    journal::commit();

    assert_eq!(journal::recover(), 1);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);
    assert_eq!(journal::recover(), 0);

    let stats = journal::stats();
    assert_eq!(stats.pending_tx_count, 0);
    assert_eq!(stats.checkpointed_sequence, stats.committed_sequence);
}

#[test]
fn journal_and_writeback_e2e_chain_runs_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_101;
    const INO: u64 = 91_001;

    journal::init();
    let sink = Arc::new(RecordingSink::new());
    writeback::register_writable_mount(MOUNT_ID, sink.clone());
    writeback::register_inode(INO, MOUNT_ID);

    let mut inode = Inode::new(INO, 0o100644);
    assert_eq!(inode.write_cached(0, b"page-0"), 6);
    assert_eq!(inode.write_cached(4096, b"page-1"), 6);
    GLOBAL_INODE_CACHE.insert(Arc::new(inode));

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 8192,
        new_mode: 0o100644,
    });
    txn.add(vfs::JournalOp::DentryCreate {
        parent_ino: 1,
        name_hash: 0xA55A,
        child_ino: INO,
    });
    txn.commit(sink.as_ref()).expect("journal transaction should commit");

    assert!(journal::begin_transaction(3).is_some());
    assert!(journal::journal_write(INO, vec![1; 32]));
    assert!(journal::journal_write(INO + 1, vec![2; 32]));
    journal::commit();

    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 2);
    assert_eq!(sink.writes.lock().expect("writes").as_slice(), &[(INO, 0, 4096), (INO, 4096, 4096)]);
    assert_eq!(sink.journal_entries.lock().expect("journal entries").len(), 3);
    assert_eq!(sink.journal_commits.load(Ordering::Relaxed), 1);
    assert!(sink.flushes.load(Ordering::Relaxed) >= 1);
    assert_eq!(journal::recover(), 2);

    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_recovery_chain_persists_journal_and_data_across_remount_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_202;
    const INO: u64 = 92_002;
    const JOURNAL_BLOCKS: u64 = 4;
    const PAGE: usize = 4096;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);
    install_cached_inode(INO, &[b"disk-page-0", b"disk-page-1"]);

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: (PAGE * 2) as u64,
        new_mode: 0o100644,
    });
    txn.add(vfs::JournalOp::DentryCreate {
        parent_ino: 1,
        name_hash: 0xD15C_BA11,
        child_ino: INO,
    });
    txn.commit(sink.as_ref())
        .expect("journal transaction should commit through writeback API");

    assert!(journal::begin_transaction(3).is_some());
    assert!(journal::journal_write(INO, vec![7; 32]));
    assert!(journal::journal_write(INO + 1, vec![8; 32]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 2);

    let data_block_4 = backing.snapshot_block(JOURNAL_BLOCKS as usize);
    let data_block_5 = backing.snapshot_block(JOURNAL_BLOCKS as usize + 1);
    let occupied_before_checkpoint = occupied_journal_slots(&backing, JOURNAL_BLOCKS);

    assert!(occupied_before_checkpoint.len() >= 2);
    assert!(occupied_before_checkpoint.iter().any(|(_, _, op)| matches!(*op, 2 | 5)));
    assert!(occupied_before_checkpoint.iter().any(|(_, _, op)| *op == 1));
    assert_eq!(&data_block_4[..11], b"disk-page-0");
    assert_eq!(&data_block_5[..11], b"disk-page-1");
    assert_eq!(journal::recover(), 2);

    let _remounted = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    let mut replay_probe = vec![0u8; PAGE];
    let mut remounted_device = backing.clone();
    remounted_device
        .read_block(occupied_before_checkpoint[0].0 as u64, &mut replay_probe)
        .expect("remounted journal block should be readable");
    assert_eq!(journal_seq(&replay_probe), occupied_before_checkpoint[0].1);

    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_recovery_survives_abort_until_checkpoint_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_203;
    const INO: u64 = 92_003;
    const JOURNAL_BLOCKS: u64 = 4;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);
    install_cached_inode(INO, &[b"abort-disk-page"]);

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 15,
        new_mode: 0o100644,
    });
    txn.commit(sink.as_ref())
        .expect("disk journal transaction should commit");

    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![9; 32]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 1);

    let occupied_before_abort = occupied_journal_slots(&backing, JOURNAL_BLOCKS);
    assert!(!occupied_before_abort.is_empty());
    journal::abort();

    let _remounted = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    let mut probe = vec![0u8; 4096];
    let mut remounted_device = backing.clone();
    remounted_device
        .read_block(occupied_before_abort[0].0 as u64, &mut probe)
        .expect("journal block should remain readable after abort");
    assert_eq!(journal_seq(&probe), occupied_before_abort[0].1);
    assert_eq!(journal::recover(), 1);

    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_recovery_handles_multiple_cycles_and_journal_slot_reuse_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_204;
    const BASE_INO: u64 = 92_100;
    const JOURNAL_BLOCKS: u64 = 3;

    journal::init();
    let backing = SharedBlockDevice::new(24);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);

    for cycle in 0..4u64 {
        let ino = BASE_INO + cycle;
        writeback::register_inode(ino, MOUNT_ID);

        let payload = format!("cycle-{cycle}-payload");
        install_cached_inode(ino, &[payload.as_bytes()]);

        let mut txn = vfs::JournalTransaction::new();
        txn.add(vfs::JournalOp::InodeUpdate {
            ino,
            new_size: payload.len() as u64,
            new_mode: 0o100644,
        });
        txn.commit(sink.as_ref()).expect("journal transaction should commit");

        assert!(journal::begin_transaction(2).is_some());
        assert!(journal::journal_write(ino, vec![cycle as u8 + 1; 16]));
        journal::commit();
        assert_eq!(writeback::fsync_inode(ino).expect("fsync should flush"), 1);
    }

    assert_eq!(journal::recover(), 4);
    for slot in 0..JOURNAL_BLOCKS as usize {
        let block = backing.snapshot_block(slot);
        assert!(journal_seq(&block) >= 1);
        assert!(matches!(journal_op(&block), 1 | 2));
    }

    let first_data = backing.snapshot_block(JOURNAL_BLOCKS as usize);
    let last_data = backing.snapshot_block(JOURNAL_BLOCKS as usize + 3);
    assert!(first_data.starts_with(b"cycle-0-payload"));
    assert!(last_data.starts_with(b"cycle-3-payload"));

    let _remounted = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    for cycle in 0..4u64 {
        GLOBAL_INODE_CACHE.evict(BASE_INO + cycle);
    }
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_checkpointed_chain_stays_quiet_across_multiple_remount_probes_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_205;
    const INO: u64 = 92_205;
    const JOURNAL_BLOCKS: u64 = 4;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);
    install_cached_inode(INO, &[b"persist-a", b"persist-b"]);

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 8192,
        new_mode: 0o100644,
    });
    txn.commit(sink.as_ref()).expect("disk journal transaction should commit");

    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![1; 16]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 2);
    assert_eq!(journal::recover(), 1);

    let before_checkpoint = occupied_journal_slots(&backing, JOURNAL_BLOCKS);
    assert!(!before_checkpoint.is_empty());
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    for _ in 0..3 {
        let _remounted = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
        assert_eq!(journal::recover(), 0);
        let after_probe = occupied_journal_slots(&backing, JOURNAL_BLOCKS);
        assert_eq!(after_probe, before_checkpoint);
    }

    let first_data = backing.snapshot_block(JOURNAL_BLOCKS as usize);
    let second_data = backing.snapshot_block(JOURNAL_BLOCKS as usize + 1);
    assert!(first_data.starts_with(b"persist-a"));
    assert!(second_data.starts_with(b"persist-b"));

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_abort_freezes_persisted_journal_image_until_checkpoint_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_206;
    const INO: u64 = 92_206;
    const JOURNAL_BLOCKS: u64 = 4;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);
    install_cached_inode(INO, &[b"freeze-me"]);

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 9,
        new_mode: 0o100644,
    });
    txn.commit(sink.as_ref()).expect("disk journal transaction should commit");

    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![5; 16]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 1);

    let occupied_before_abort = occupied_journal_slots(&backing, JOURNAL_BLOCKS);
    journal::abort();
    assert!(!journal::journal_write(INO + 1, vec![6; 16]));
    assert_eq!(occupied_journal_slots(&backing, JOURNAL_BLOCKS), occupied_before_abort);
    assert_eq!(journal::recover(), 1);

    let _remounted = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    assert_eq!(occupied_journal_slots(&backing, JOURNAL_BLOCKS), occupied_before_abort);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_reboot_chain_preserves_latest_persisted_payload_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_207;
    const INO: u64 = 92_207;
    const JOURNAL_BLOCKS: u64 = 4;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);

    install_cached_inode(INO, &[b"first-payload"]);
    let mut txn1 = vfs::JournalTransaction::new();
    txn1.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 13,
        new_mode: 0o100644,
    });
    txn1.commit(sink.as_ref()).expect("first transaction should commit");
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![1; 16]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("first fsync should flush"), 1);
    assert_eq!(journal::recover(), 1);
    assert_block_prefix(&backing, JOURNAL_BLOCKS as usize, b"first-payload");

    let _reboot_1 = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    install_cached_inode(INO, &[b"second-payload"]);
    let mut txn2 = vfs::JournalTransaction::new();
    txn2.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 14,
        new_mode: 0o100644,
    });
    txn2.commit(sink.as_ref()).expect("second transaction should commit");
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![2; 16]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("second fsync should flush"), 1);
    assert_eq!(journal::recover(), 1);

    let _reboot_2 = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
    assert_block_prefix(&backing, JOURNAL_BLOCKS as usize, b"second-payload");
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}

#[test]
fn disk_backed_recovery_remains_quiet_after_double_boot_without_new_writes_on_host() {
    let _guard = lock_vfs();

    const MOUNT_ID: usize = 9_208;
    const INO: u64 = 92_208;
    const JOURNAL_BLOCKS: u64 = 4;

    journal::init();
    let backing = SharedBlockDevice::new(16);
    let sink = mount_disk_sink(MOUNT_ID, &backing, JOURNAL_BLOCKS);
    writeback::register_inode(INO, MOUNT_ID);
    install_cached_inode(INO, &[b"steady-payload"]);

    let mut txn = vfs::JournalTransaction::new();
    txn.add(vfs::JournalOp::InodeUpdate {
        ino: INO,
        new_size: 14,
        new_mode: 0o100644,
    });
    txn.commit(sink.as_ref()).expect("transaction should commit");
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(INO, vec![3; 16]));
    journal::commit();
    assert_eq!(writeback::fsync_inode(INO).expect("fsync should flush"), 1);
    assert_eq!(journal::recover(), 1);

    let occupied_before = occupied_journal_slots(&backing, JOURNAL_BLOCKS);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    for _ in 0..2 {
        let _reboot = BlockWritebackSink::new(Box::new(backing.clone()), JOURNAL_BLOCKS);
        assert_eq!(journal::recover(), 0);
        assert_eq!(occupied_journal_slots(&backing, JOURNAL_BLOCKS), occupied_before);
        assert_block_prefix(&backing, JOURNAL_BLOCKS as usize, b"steady-payload");
    }

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT_ID).expect("unregister should succeed");
}
