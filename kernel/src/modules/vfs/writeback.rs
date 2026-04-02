//! VFS Writeback Engine
//!
//! Manages dirty page tracking, periodic writeback, and fsync ordering.
//! Provides write-ahead journaling for crash consistency.
//!
//! Design:
//! - Each mounted writable filesystem registers a `WritebackSink`.
//! - Dirty pages are queued into a global writeback list.
//! - A periodic writeback pass flushes oldest-first up to a budget.
//! - `fsync()` forces immediate flush of a specific inode's dirty pages.
//! - A simple write-ahead journal ensures metadata consistency.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use aethercore_common::units::PAGE_SIZE_4K;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use super::cache::GLOBAL_INODE_CACHE;
use super::writeback_support::{dirty_keys_for_inode, remove_dirty_keys_for_inode};

// ── Configuration ────────────────────────────────────────────────────────────

/// Maximum dirty pages before writeback pressure triggers an eager flush.
const DIRTY_PAGE_HIGH_WATERMARK: usize = 4096;
/// Target dirty page count after an eager flush.
const DIRTY_PAGE_LOW_WATERMARK: usize = 1024;
/// Flush batch size used when pressure watermark is exceeded.
const DIRTY_PAGE_PRESSURE_FLUSH_BATCH: usize = DIRTY_PAGE_HIGH_WATERMARK - DIRTY_PAGE_LOW_WATERMARK;
/// Maximum pages flushed per periodic writeback pass.
const WRITEBACK_BUDGET_PER_PASS: usize = 256;
/// Page size (must match cache.rs).
#[allow(dead_code)]
const PAGE_SIZE: usize = PAGE_SIZE_4K;

// ── Writeback Sink Trait ─────────────────────────────────────────────────────

/// A sink that can accept flushed pages and persist them to stable storage.
/// Each writable filesystem backend implements this.
pub trait WritebackSink: Send + Sync {
    /// Write a single page's data to the backing store.
    /// `ino`: inode number, `offset`: byte offset within the file, `data`: page content.
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str>;

    /// Flush all volatile caches to stable storage (write barrier).
    fn flush(&self) -> Result<(), &'static str>;

    /// Write a journal entry before committing a metadata operation.
    fn journal_write(&self, entry: &JournalEntry) -> Result<(), &'static str> {
        let _ = entry;
        Ok(()) // Default: no journaling
    }

    /// Commit the journal (mark all pending entries as committed).
    fn journal_commit(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

// ── Journal ──────────────────────────────────────────────────────────────────

/// Journal entry types for write-ahead logging.
#[derive(Debug, Clone)]
pub enum JournalOp {
    /// Inode metadata update (size, mode, timestamps).
    InodeUpdate {
        ino: u64,
        new_size: u64,
        new_mode: u16,
    },
    /// Block allocation for an inode.
    BlockAlloc {
        ino: u64,
        logical_block: u64,
        physical_block: u64,
    },
    /// Block deallocation.
    BlockFree { physical_block: u64 },
    /// Directory entry create.
    DentryCreate {
        parent_ino: u64,
        name_hash: u64,
        child_ino: u64,
    },
    /// Directory entry remove.
    DentryRemove { parent_ino: u64, name_hash: u64 },
    /// Transaction commit marker.
    Commit { txn_id: u64 },
}

/// A single journal entry with a sequence number.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub seq: u64,
    pub op: JournalOp,
}

/// Transaction for grouping multiple journal operations atomically.
pub struct JournalTransaction {
    entries: Vec<JournalOp>,
    txn_id: u64,
}

static NEXT_TXN_ID: AtomicU64 = AtomicU64::new(1);

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

    /// Commit the transaction to the given sink.
    /// Writes all entries + a commit marker, then calls flush.
    pub fn commit(self, sink: &dyn WritebackSink) -> Result<(), &'static str> {
        let entry_count = self.entries.len() as u64;
        let seq_base = NEXT_JOURNAL_SEQ.fetch_add(entry_count + 1, Ordering::Relaxed);

        for (i, op) in self.entries.into_iter().enumerate() {
            let entry = JournalEntry {
                seq: seq_base + i as u64,
                op,
            };
            sink.journal_write(&entry)?;
        }

        // Commit marker
        let commit_entry = JournalEntry {
            seq: seq_base + entry_count,
            op: JournalOp::Commit {
                txn_id: self.txn_id,
            },
        };
        sink.journal_write(&commit_entry)?;
        sink.journal_commit()?;
        Ok(())
    }
}

impl Default for JournalTransaction {
    fn default() -> Self {
        Self::new()
    }
}

static NEXT_JOURNAL_SEQ: AtomicU64 = AtomicU64::new(1);

// ── Dirty Page Tracker ───────────────────────────────────────────────────────

/// Identifies a dirty page: (inode number, page index within file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DirtyPageKey {
    pub(super) ino: u64,
    pub(super) page_idx: u64,
}

/// Entry in the dirty page list with age tracking.
#[derive(Debug, Clone, Copy)]
pub(super) struct DirtyPageEntry {
    /// Tick when the page was first dirtied.
    pub(super) dirty_since: u64,
    /// Number of times this page has been re-dirtied without flush.
    pub(super) redirty_count: u32,
}

// ── Writeback Manager ────────────────────────────────────────────────────────

/// Global writeback manager. Coordinates dirty page tracking and flush scheduling.
pub struct WritebackManager {
    /// Registered sinks indexed by mount_id.
    sinks: BTreeMap<usize, Arc<dyn WritebackSink>>,
    /// Inode-to-mount mapping (so we know which sink to flush to).
    ino_to_mount: BTreeMap<u64, usize>,
    /// Global dirty page tracker.
    dirty_pages: BTreeMap<DirtyPageKey, DirtyPageEntry>,
    /// Current global tick (updated by periodic timer).
    current_tick: u64,
}

/// Statistics for telemetry.
#[derive(Debug, Clone, Copy, Default)]
pub struct WritebackStats {
    pub total_flushes: u64,
    pub total_pages_written: u64,
    pub total_fsync_calls: u64,
    pub total_journal_commits: u64,
    pub dirty_page_count: usize,
    pub pressure_flushes: u64,
}

static STATS_TOTAL_FLUSHES: AtomicU64 = AtomicU64::new(0);
static STATS_PAGES_WRITTEN: AtomicU64 = AtomicU64::new(0);
static STATS_FSYNC_CALLS: AtomicU64 = AtomicU64::new(0);
static STATS_JOURNAL_COMMITS: AtomicU64 = AtomicU64::new(0);
static STATS_PRESSURE_FLUSHES: AtomicU64 = AtomicU64::new(0);

pub fn writeback_stats() -> WritebackStats {
    let mgr = GLOBAL_WRITEBACK.lock();
    WritebackStats {
        total_flushes: STATS_TOTAL_FLUSHES.load(Ordering::Relaxed),
        total_pages_written: STATS_PAGES_WRITTEN.load(Ordering::Relaxed),
        total_fsync_calls: STATS_FSYNC_CALLS.load(Ordering::Relaxed),
        total_journal_commits: STATS_JOURNAL_COMMITS.load(Ordering::Relaxed),
        dirty_page_count: mgr.dirty_pages.len(),
        pressure_flushes: STATS_PRESSURE_FLUSHES.load(Ordering::Relaxed),
    }
}

impl WritebackManager {
    const fn new() -> Self {
        Self {
            sinks: BTreeMap::new(),
            ino_to_mount: BTreeMap::new(),
            dirty_pages: BTreeMap::new(),
            current_tick: 0,
        }
    }

    /// Register a writable filesystem sink.
    pub fn register_sink(&mut self, mount_id: usize, sink: Arc<dyn WritebackSink>) {
        self.sinks.insert(mount_id, sink);
    }

    /// Unregister a sink (on unmount). All dirty pages for this mount are flushed first.
    pub fn unregister_sink(&mut self, mount_id: usize) -> Result<(), &'static str> {
        // Flush all dirty pages belonging to this mount
        self.flush_mount(mount_id)?;
        self.sinks.remove(&mount_id);
        // Remove inode mappings for this mount
        self.ino_to_mount.retain(|_, &mut mid| mid != mount_id);
        Ok(())
    }

    /// Associate an inode with a mount point.
    pub fn register_inode(&mut self, ino: u64, mount_id: usize) {
        self.ino_to_mount.insert(ino, mount_id);
    }

    /// Mark a page as dirty.
    pub fn mark_dirty(&mut self, ino: u64, page_idx: u64) {
        let key = DirtyPageKey { ino, page_idx };
        let entry = self.dirty_pages.entry(key).or_insert(DirtyPageEntry {
            dirty_since: self.current_tick,
            redirty_count: 0,
        });
        entry.redirty_count += 1;

        // Check pressure
        if self.dirty_pages.len() > DIRTY_PAGE_HIGH_WATERMARK {
            STATS_PRESSURE_FLUSHES.fetch_add(1, Ordering::Relaxed);
            let _ = self.flush_oldest(DIRTY_PAGE_PRESSURE_FLUSH_BATCH);
        }
    }

    /// Periodic writeback: flush up to `WRITEBACK_BUDGET_PER_PASS` oldest dirty pages.
    pub fn periodic_writeback(&mut self, tick: u64) -> usize {
        self.current_tick = tick;
        match self.flush_oldest(WRITEBACK_BUDGET_PER_PASS) {
            Ok(n) => n,
            Err(_) => 0,
        }
    }

    /// Flush all dirty pages for a specific inode (fsync).
    pub fn fsync_inode(&mut self, ino: u64) -> Result<usize, &'static str> {
        STATS_FSYNC_CALLS.fetch_add(1, Ordering::Relaxed);

        let mount_id = self
            .ino_to_mount
            .get(&ino)
            .copied()
            .ok_or("inode not associated with any mount")?;
        let sink = self
            .sinks
            .get(&mount_id)
            .ok_or("mount sink not found")?
            .clone();
        let inode = GLOBAL_INODE_CACHE.get(ino).ok_or("inode not in cache")?;

        let flushed = self.flush_inode_pages(&sink, &inode, ino, true)?;

        // Issue a write barrier to ensure durability
        sink.flush()?;

        STATS_PAGES_WRITTEN.fetch_add(flushed as u64, Ordering::Relaxed);
        STATS_TOTAL_FLUSHES.fetch_add(1, Ordering::Relaxed);
        Ok(flushed)
    }

    /// Flush all dirty pages for all inodes on a specific mount.
    fn flush_mount(&mut self, mount_id: usize) -> Result<(), &'static str> {
        let sink = self
            .sinks
            .get(&mount_id)
            .ok_or("mount sink not found")?
            .clone();

        // Find all inodes on this mount
        let inos: Vec<u64> = self
            .ino_to_mount
            .iter()
            .filter(|&(_, &mid)| mid == mount_id)
            .map(|(&ino, _)| ino)
            .collect();

        for ino in &inos {
            if let Some(inode) = GLOBAL_INODE_CACHE.get(*ino) {
                let _ = self.flush_inode_pages(&sink, &inode, *ino, false)?;
            } else {
                // If inode has been evicted, stale dirty keys must still be removed.
                remove_dirty_keys_for_inode(&mut self.dirty_pages, *ino);
            }
        }

        sink.flush()?;
        Ok(())
    }

    fn flush_inode_pages(
        &mut self,
        sink: &Arc<dyn WritebackSink>,
        inode: &Arc<super::cache::Inode>,
        ino: u64,
        strict_errors: bool,
    ) -> Result<usize, &'static str> {
        let keys = dirty_keys_for_inode(&self.dirty_pages, ino);
        let cache = inode.pages.lock();
        let mut flushed = 0usize;

        for key in &keys {
            if let Some(page_arc) = cache.get(&key.page_idx) {
                let mut page = page_arc.lock();
                if page.dirty {
                    if strict_errors {
                        sink.write_page(ino, page.offset, &page.data)?;
                        page.dirty = false;
                        flushed += 1;
                    } else if sink.write_page(ino, page.offset, &page.data).is_ok() {
                        page.dirty = false;
                        flushed += 1;
                    }
                }
            }
            self.dirty_pages.remove(key);
        }

        Ok(flushed)
    }

    /// Flush the N oldest dirty pages across all mounts.
    fn flush_oldest(&mut self, budget: usize) -> Result<usize, &'static str> {
        // Sort by dirty_since ascending (oldest first)
        let mut candidates: Vec<(DirtyPageKey, DirtyPageEntry)> =
            self.dirty_pages.iter().map(|(&k, &v)| (k, v)).collect();
        candidates.sort_by_key(|(_, e)| e.dirty_since);
        candidates.truncate(budget);

        let mut flushed = 0usize;
        let mut flush_needed: BTreeMap<usize, bool> = BTreeMap::new();

        for (key, _) in &candidates {
            let mount_id = match self.ino_to_mount.get(&key.ino) {
                Some(&mid) => mid,
                None => continue,
            };
            let sink = match self.sinks.get(&mount_id) {
                Some(s) => s.clone(),
                None => continue,
            };

            if let Some(inode) = GLOBAL_INODE_CACHE.get(key.ino) {
                let cache = inode.pages.lock();
                if let Some(page_arc) = cache.get(&key.page_idx) {
                    let mut page = page_arc.lock();
                    if page.dirty {
                        if sink.write_page(key.ino, page.offset, &page.data).is_ok() {
                            page.dirty = false;
                            flushed += 1;
                            flush_needed.insert(mount_id, true);
                        }
                    }
                }
            }
            self.dirty_pages.remove(key);
        }

        // Barrier each affected mount
        for (mount_id, _) in &flush_needed {
            if let Some(sink) = self.sinks.get(mount_id) {
                let _ = sink.flush();
            }
        }

        STATS_PAGES_WRITTEN.fetch_add(flushed as u64, Ordering::Relaxed);
        if flushed > 0 {
            STATS_TOTAL_FLUSHES.fetch_add(1, Ordering::Relaxed);
        }
        Ok(flushed)
    }

    /// Sync all mounts (sync(2) equivalent).
    pub fn sync_all(&mut self) -> Result<usize, &'static str> {
        let mount_ids: Vec<usize> = self.sinks.keys().copied().collect();
        let mut total = 0;
        for mid in mount_ids {
            self.flush_mount(mid)?;
            total += 1;
        }
        Ok(total)
    }
}

/// Global writeback manager instance.
pub static GLOBAL_WRITEBACK: Mutex<WritebackManager> = Mutex::new(WritebackManager::new());

// ── Public API ───────────────────────────────────────────────────────────────

/// Register a writable filesystem with the writeback engine.
pub fn register_writable_mount(mount_id: usize, sink: Arc<dyn WritebackSink>) {
    GLOBAL_WRITEBACK.lock().register_sink(mount_id, sink);
}

/// Unregister a mount (flushes all dirty data first).
pub fn unregister_writable_mount(mount_id: usize) -> Result<(), &'static str> {
    GLOBAL_WRITEBACK.lock().unregister_sink(mount_id)
}

/// Register an inode→mount association.
pub fn register_inode(ino: u64, mount_id: usize) {
    GLOBAL_WRITEBACK.lock().register_inode(ino, mount_id);
}

/// Mark a page dirty (called from Inode::write_cached).
pub fn mark_dirty(ino: u64, page_idx: u64) {
    GLOBAL_WRITEBACK.lock().mark_dirty(ino, page_idx);
}

/// fsync an inode — flush all dirty pages to stable storage.
pub fn fsync_inode(ino: u64) -> Result<usize, &'static str> {
    GLOBAL_WRITEBACK.lock().fsync_inode(ino)
}

/// Periodic writeback tick (called from the timer interrupt or a kernel thread).
pub fn periodic_writeback(tick: u64) -> usize {
    GLOBAL_WRITEBACK.lock().periodic_writeback(tick)
}

/// sync(2) — flush all dirty data for all mounts.
pub fn sync_all() -> Result<usize, &'static str> {
    GLOBAL_WRITEBACK.lock().sync_all()
}

#[cfg(test)]
fn reset_writeback_state_for_tests() {
    *GLOBAL_WRITEBACK.lock() = WritebackManager::new();
    STATS_TOTAL_FLUSHES.store(0, Ordering::Relaxed);
    STATS_PAGES_WRITTEN.store(0, Ordering::Relaxed);
    STATS_FSYNC_CALLS.store(0, Ordering::Relaxed);
    STATS_JOURNAL_COMMITS.store(0, Ordering::Relaxed);
    STATS_PRESSURE_FLUSHES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn evict_inodes_for_tests(inodes: &[u64]) {
    for ino in inodes {
        super::cache::GLOBAL_INODE_CACHE.evict(*ino);
    }
}

#[cfg(test)]
#[path = "writeback/tests.rs"]
mod tests;
