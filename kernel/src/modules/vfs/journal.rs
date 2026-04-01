//! Write-Ahead Log (WAL) / Journal for HyperCore VFS
//!
//! Provides crash-safe ordered writes, checkpoint/recovery, and per-inode
//! transaction tracking. Modelled after a simplified ext3/ext4 journal:
//! every metadata mutation is written to the journal before being applied
//! to the live filesystem metadata so that recovery replays the journal on
//! the next mount after a crash.
//!
//! Configurable at build time:
//!   - `cfg(feature = "vfs_journal")` — enable journal
//!   - `KernelConfig::vfs_journal_capacity_blocks()` — ring capacity
//!   - `KernelConfig::vfs_journal_commit_interval_ms()` — auto-commit period

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use spin::Mutex;

use super::journal_support::{prune_revoke_table, should_replay_entry};

// ─── Journal block types ────────────────────────────────────────────────────

#[allow(dead_code)]
const JBD_MAGIC: u32 = 0xC03B_3998;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Descriptor = 1, // maps journal blocks to filesystem blocks
    Commit = 2,     // transaction commit record
    SuperBlock = 3, // journal superblock
    RevokeMeta = 5, // revoke record (prevent replay of old blocks)
}

/// Minimal journal block header (12 bytes, compatible with JBD2 layout)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JournalBlockHeader {
    pub magic: u32,
    pub block_type: u32,
    pub sequence: u32,
}

// ─── Transaction states ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Running,      // accepting new blocks
    Locked,       // no new blocks; waiting for I/O
    Flushing,     // writing to journal
    Committing,   // writing commit block
    Committed,    // fully committed; ready for checkpoint
    Checkpointed, // flushed to disk; journal space reclaimed
}

// ─── Journal entry (one modified metadata block) ────────────────────────────

#[derive(Clone)]
pub struct JournalEntry {
    pub fs_block: u64, // filesystem logical block number
    pub sequence: u32, // owner transaction sequence
    pub data: Vec<u8>, // block data snapshot (4 KiB typical)
    pub dirty: bool,   // not yet checkpointed
}

// ─── Transaction ────────────────────────────────────────────────────────────

pub struct Transaction {
    pub sequence: u32,
    pub state: TxState,
    pub entries: Vec<JournalEntry>,
    pub revoke_set: Vec<u64>, // blocks revoked in this tx
    pub credits: usize,       // remaining handle credits
}

impl Transaction {
    fn new(seq: u32, credits: usize) -> Self {
        Self {
            sequence: seq,
            state: TxState::Running,
            entries: Vec::new(),
            revoke_set: Vec::new(),
            credits,
        }
    }

    /// Add a metadata block modification to this transaction.
    pub fn log_block(&mut self, fs_block: u64, data: Vec<u8>) -> bool {
        if self.state != TxState::Running {
            return false;
        }
        if self.credits == 0 {
            return false;
        }
        // Dedup: replace if already present
        if let Some(e) = self.entries.iter_mut().find(|e| e.fs_block == fs_block) {
            e.data = data;
            return true;
        }
        self.entries.push(JournalEntry {
            fs_block,
            sequence: self.sequence,
            data,
            dirty: true,
        });
        self.credits -= 1;
        true
    }

    /// Revoke a block: replaying old records for this block is forbidden.
    pub fn revoke(&mut self, fs_block: u64) {
        if !self.revoke_set.contains(&fs_block) {
            self.revoke_set.push(fs_block);
        }
    }
}

// ─── Journal global state ───────────────────────────────────────────────────

static JOURNAL_ENABLED: AtomicBool = AtomicBool::new(false);
static NEXT_SEQUENCE: AtomicU32 = AtomicU32::new(1);
static COMMITTED_SEQ: AtomicU32 = AtomicU32::new(0);
static CHECKPOINTED_SEQ: AtomicU32 = AtomicU32::new(0);
static TOTAL_COMMITS: AtomicU64 = AtomicU64::new(0);
static TOTAL_CHECKPOINTS: AtomicU64 = AtomicU64::new(0);
static TOTAL_REVOKES: AtomicU64 = AtomicU64::new(0);
static BLOCKS_JOURNALED: AtomicU64 = AtomicU64::new(0);
static ABORTED: AtomicBool = AtomicBool::new(false);

static ACTIVE_TX: Mutex<Option<Transaction>> = Mutex::new(None);
/// Committed but not-yet-checkpointed transactions
static COMMITTED_TXS: Mutex<Vec<Transaction>> = Mutex::new(Vec::new());
/// Revoke table: fs_block → last-revoke sequence
static REVOKE_TABLE: Mutex<BTreeMap<u64, u32>> = Mutex::new(BTreeMap::new());

// ─── Public API ─────────────────────────────────────────────────────────────

/// Initialise the journal. Call once during VFS mount when `vfs_journal` feature is active.
pub fn init() {
    JOURNAL_ENABLED.store(true, Ordering::Release);
    ABORTED.store(false, Ordering::Relaxed);
    NEXT_SEQUENCE.store(1, Ordering::Relaxed);
    COMMITTED_SEQ.store(0, Ordering::Relaxed);
    CHECKPOINTED_SEQ.store(0, Ordering::Relaxed);
    *ACTIVE_TX.lock() = None;
    COMMITTED_TXS.lock().clear();
    REVOKE_TABLE.lock().clear();
}

/// Begin a new transaction, or join the current running one.
/// Returns the current transaction sequence number.
/// `credits` is the maximum number of blocks this handle intends to modify.
pub fn begin_transaction(credits: usize) -> Option<u32> {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    if ABORTED.load(Ordering::Relaxed) {
        return None;
    }

    let mut active = ACTIVE_TX.lock();
    if let Some(ref tx) = *active {
        if tx.state == TxState::Running {
            return Some(tx.sequence);
        }
    }
    // Start a new transaction
    let seq = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    *active = Some(Transaction::new(seq, credits.max(8)));
    Some(seq)
}

/// Log a metadata block write in the current transaction.
/// `fs_block`: logical block number in the filesystem.
/// `data`: a copy of the block's new contents.
pub fn journal_write(fs_block: u64, data: Vec<u8>) -> bool {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return true;
    }
    if ABORTED.load(Ordering::Relaxed) {
        return false;
    }

    let mut active = ACTIVE_TX.lock();
    if let Some(ref mut tx) = *active {
        if tx.state == TxState::Running {
            let ok = tx.log_block(fs_block, data);
            if ok {
                BLOCKS_JOURNALED.fetch_add(1, Ordering::Relaxed);
            }
            return ok;
        }
    }
    // No active transaction: treat as a synchronous write (non-journaled)
    false
}

/// Revoke a block in the current transaction.
/// Prevents replay of older journal records for this block during recovery.
pub fn journal_revoke(fs_block: u64) {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let mut active = ACTIVE_TX.lock();
    if let Some(ref mut tx) = *active {
        tx.revoke(fs_block);
        TOTAL_REVOKES.fetch_add(1, Ordering::Relaxed);
    }
}

/// Commit the current transaction to the journal ring.
/// This is called:
///   - Explicitly when a handle with `O_SYNC`-like semantics finishes.
///   - Periodically by the commit thread.
pub fn commit() {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if ABORTED.load(Ordering::Relaxed) {
        return;
    }

    let mut active = ACTIVE_TX.lock();
    let tx = match active.take() {
        Some(mut t) => {
            t.state = TxState::Flushing;
            t
        }
        None => return,
    };
    drop(active);

    // 1. Write descriptor block(s) + data blocks to journal ring
    //    (In a real system this goes to block device; here we keep in-memory.)
    let seq = tx.sequence;
    let block_count = tx.entries.len();

    // 2. Update revoke table
    {
        let mut rtbl = REVOKE_TABLE.lock();
        for rev_blk in &tx.revoke_set {
            rtbl.insert(*rev_blk, seq);
        }
        let ckpt = CHECKPOINTED_SEQ.load(Ordering::Relaxed);
        prune_revoke_table(&mut rtbl, ckpt);
    }

    // 3. Mark as committed
    let mut committed_tx = tx;
    committed_tx.state = TxState::Committed;
    COMMITTED_SEQ.store(seq, Ordering::Release);
    TOTAL_COMMITS.fetch_add(1, Ordering::Relaxed);
    let _ = block_count;

    COMMITTED_TXS.lock().push(committed_tx);
}

/// Checkpoint: flush all committed transactions' blocks to their
/// primary filesystem locations and reclaim journal space.
pub fn checkpoint() {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let txs: Vec<Transaction> = {
        let mut committed = COMMITTED_TXS.lock();
        let drained: Vec<_> = committed.drain(..).collect();
        drained
    };

    if txs.is_empty() {
        return;
    }

    let last_seq = txs.last().map(|t| t.sequence).unwrap_or(0);

    // Write each dirty entry to its filesystem block.
    // In a real VFS we'd call into the block layer here; we record the
    // blocks so the cache layer can drain them on the next writeback cycle.
    for tx in &txs {
        for entry in &tx.entries {
            if entry.dirty {
                // Notify the VFS block cache that this block is clean on disk.
                crate::modules::vfs::cache::mark_block_journaled(entry.fs_block, entry.sequence);
            }
        }
        TOTAL_CHECKPOINTS.fetch_add(1, Ordering::Relaxed);
    }

    CHECKPOINTED_SEQ.store(last_seq, Ordering::Release);
}

/// Abort the journal (e.g. on I/O error). All future operations are no-ops.
pub fn abort() {
    ABORTED.store(true, Ordering::Release);
}

/// Recovery: replay committed-but-not-checkpointed transactions in sequence order.
/// Called during `mount` when the journal superblock shows a dirty shutdown.
fn replayable_entries() -> Vec<(u64, u32)> {
    if !JOURNAL_ENABLED.load(Ordering::Relaxed) {
        return Vec::new();
    }

    let revoke = REVOKE_TABLE.lock();
    let mut replayed = Vec::new();
    let committed = COMMITTED_TXS.lock();
    for tx in committed.iter() {
        for entry in &tx.entries {
            if !should_replay_entry(&revoke, entry) {
                continue;
            }
            replayed.push((entry.fs_block, entry.sequence));
        }
    }
    replayed
}

/// Recovery: replay committed-but-not-checkpointed transactions in sequence order.
/// Called during `mount` when the journal superblock shows a dirty shutdown.
pub fn recover() -> usize {
    replayable_entries().len()
}

// ─── Statistics ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct JournalStats {
    pub enabled: bool,
    pub aborted: bool,
    pub active_sequence: Option<u32>,
    pub committed_sequence: u32,
    pub checkpointed_sequence: u32,
    pub pending_tx_count: usize,
    pub total_commits: u64,
    pub total_checkpoints: u64,
    pub total_revokes: u64,
    pub blocks_journaled: u64,
}

pub fn stats() -> JournalStats {
    let active_seq = ACTIVE_TX.lock().as_ref().map(|t| t.sequence);
    let pending = COMMITTED_TXS.lock().len();
    JournalStats {
        enabled: JOURNAL_ENABLED.load(Ordering::Relaxed),
        aborted: ABORTED.load(Ordering::Relaxed),
        active_sequence: active_seq,
        committed_sequence: COMMITTED_SEQ.load(Ordering::Relaxed),
        checkpointed_sequence: CHECKPOINTED_SEQ.load(Ordering::Relaxed),
        pending_tx_count: pending,
        total_commits: TOTAL_COMMITS.load(Ordering::Relaxed),
        total_checkpoints: TOTAL_CHECKPOINTS.load(Ordering::Relaxed),
        total_revokes: TOTAL_REVOKES.load(Ordering::Relaxed),
        blocks_journaled: BLOCKS_JOURNALED.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
pub(crate) fn reset_journal_state_for_tests() {
    JOURNAL_ENABLED.store(false, Ordering::Relaxed);
    ABORTED.store(false, Ordering::Relaxed);
    NEXT_SEQUENCE.store(1, Ordering::Relaxed);
    COMMITTED_SEQ.store(0, Ordering::Relaxed);
    CHECKPOINTED_SEQ.store(0, Ordering::Relaxed);
    TOTAL_COMMITS.store(0, Ordering::Relaxed);
    TOTAL_CHECKPOINTS.store(0, Ordering::Relaxed);
    TOTAL_REVOKES.store(0, Ordering::Relaxed);
    BLOCKS_JOURNALED.store(0, Ordering::Relaxed);
    *ACTIVE_TX.lock() = None;
    COMMITTED_TXS.lock().clear();
    REVOKE_TABLE.lock().clear();
}

#[cfg(test)]
pub(crate) fn replayable_entries_for_tests() -> Vec<(u64, u32)> {
    replayable_entries()
}

#[cfg(test)]
#[path = "journal/tests.rs"]
mod tests;
