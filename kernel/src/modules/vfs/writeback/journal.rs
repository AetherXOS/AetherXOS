use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::WritebackSink;

/// Journal entry types for write-ahead logging.
#[derive(Debug, Clone)]
pub enum JournalOp {
    InodeUpdate { ino: u64, new_size: u64, new_mode: u16 },
    BlockAlloc { ino: u64, logical_block: u64, physical_block: u64 },
    BlockFree { physical_block: u64 },
    DentryCreate { parent_ino: u64, name_hash: u64, child_ino: u64 },
    DentryRemove { parent_ino: u64, name_hash: u64 },
    Commit { txn_id: u64 },
}

pub struct JournalEntry {
    pub seq: u64,
    pub op: JournalOp,
}

pub struct JournalTransaction {
    pub entries: Vec<JournalOp>,
    pub txn_id: u64,
}

static NEXT_TXN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_JOURNAL_SEQ: AtomicU64 = AtomicU64::new(1);

impl JournalTransaction {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            txn_id: NEXT_TXN_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn add(&mut self, op: JournalOp) {
        self.entries.push(op);
    }

    pub fn commit(self, sink: &dyn WritebackSink) -> Result<(), &'static str> {
        let entry_count = self.entries.len() as u64;
        let seq_base = NEXT_JOURNAL_SEQ.fetch_add(entry_count + 1, Ordering::Relaxed);

        for (i, op) in self.entries.into_iter().enumerate() {
            let entry = JournalEntry { seq: seq_base + i as u64, op };
            sink.journal_write(&entry)?;
        }

        let commit_entry = JournalEntry {
            seq: seq_base + entry_count,
            op: JournalOp::Commit { txn_id: self.txn_id },
        };
        sink.journal_write(&commit_entry)?;
        sink.journal_commit()?;
        Ok(())
    }
}
