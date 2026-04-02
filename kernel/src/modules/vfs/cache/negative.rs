use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Caches ENOENT results so repeated lookups of non-existent paths can be
/// answered without hitting the backing filesystem.
///
/// Entries are evicted in LRU order when the cache reaches capacity.
pub struct NegativeDentryCache {
    /// path -> tick-of-insertion (for LRU eviction).
    entries: Mutex<BTreeMap<String, u64>>,
    max_entries: usize,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl NegativeDentryCache {
    pub const fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            max_entries,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    /// Record a negative lookup result for `path`.
    pub fn insert(&self, path: String, tick: u64) {
        let mut map = self.entries.lock();
        if map.len() >= self.max_entries {
            // Evict oldest entry.
            if let Some(oldest_key) = map
                .iter()
                .min_by_key(|&(_, &v)| v)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest_key);
            }
        }
        map.insert(path, tick);
    }

    /// Check if `path` is in the negative cache.
    pub fn lookup(&self, path: &str) -> bool {
        let map = self.entries.lock();
        if map.contains_key(path) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Invalidate a negative entry (e.g. after a file is created at that path).
    pub fn invalidate(&self, path: &str) {
        self.entries.lock().remove(path);
    }

    /// Invalidate all entries under a directory prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut map = self.entries.lock();
        let keys: alloc::vec::Vec<String> = map
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        for k in keys {
            map.remove(&k);
        }
    }

    /// Cache statistics.
    pub fn stats(&self) -> (usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

/// Called by the journal checkpoint to record that a journaled block has been
/// written to its primary filesystem location. In a full implementation this
/// would mark the corresponding cache page as clean; for now it is a no-op
/// placeholder for future integration.
pub fn mark_block_journaled(_block: u64, _seq: u32) {
    // No-op: production code would clear the dirty bit in the block cache.
}

/// Global negative dentry cache (max 4096 entries).
pub static NEGATIVE_DENTRY_CACHE: NegativeDentryCache = NegativeDentryCache::new(4096);
