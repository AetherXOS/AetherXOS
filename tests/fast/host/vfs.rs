use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};

use aethercore::modules::vfs::{
    journal,
    writeback::{self, WritebackSink},
    Inode, JournalEntry, GLOBAL_INODE_CACHE,
};
use serial_test::serial;

fn vfslock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct Sink {
    writes: Mutex<Vec<(u64, u64, usize)>>,
    flushes: AtomicU64,
}

impl Sink {
    fn new() -> Self {
        Self {
            writes: Mutex::new(Vec::new()),
            flushes: AtomicU64::new(0),
        }
    }
}

impl WritebackSink for Sink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        self.writes
            .lock()
            .expect("writes")
            .push((ino, offset, data.len()));
        Ok(())
    }

    fn flush(&self) -> Result<(), &'static str> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn journal_write(&self, _entry: &JournalEntry) -> Result<(), &'static str> {
        Ok(())
    }

    fn journal_commit(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

#[test]
#[serial]
fn journal_recovery_stays_stable_after_checkpoint() {
    let _guard = vfslock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    journal::init();
    assert!(journal::begin_transaction(2).is_some());
    assert!(journal::journal_write(4001, vec![4; 8]));
    journal::commit();

    assert_eq!(journal::recover(), 1);
    journal::checkpoint();
    assert_eq!(journal::recover(), 0);

    let stats = journal::stats();
    assert_eq!(stats.pending_tx_count, 0);
    assert_eq!(stats.checkpointed_sequence, stats.committed_sequence);
}

#[test]
#[serial]
fn writeback_flushes_cached_pages_through_registered_sink() {
    let _guard = vfslock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    const MOUNT: usize = 18_201;
    const INO: u64 = 81_001;

    let sink = Arc::new(Sink::new());
    writeback::register_writable_mount(MOUNT, sink.clone());
    writeback::register_inode(INO, MOUNT);

    let mut inode = Inode::new(INO, 0o100644);
    assert_eq!(inode.write_cached(0, b"page-data"), 9);
    GLOBAL_INODE_CACHE.insert(Arc::new(inode));

    let flushed = writeback::fsync_inode(INO).expect("fsync should succeed");
    assert_eq!(flushed, 1);
    assert_eq!(
        sink.writes.lock().expect("writes").as_slice(),
        &[(INO, 0, 4096)]
    );
    assert!(sink.flushes.load(Ordering::Relaxed) >= 1);

    GLOBAL_INODE_CACHE.evict(INO);
    writeback::unregister_writable_mount(MOUNT).expect("mount should unregister");
}
