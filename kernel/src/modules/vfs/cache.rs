use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use aethercore_common::units::PAGE_SIZE_4K;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

// ── Page Cache ────────────────────────────────────────────────────────────────

/// The kernel page size used by the VFS page cache.
const PAGE_SIZE: usize = PAGE_SIZE_4K;

/// One 4 KiB page in the VFS Page Cache.
pub struct CachePage {
    /// Byte offset within the file (always a multiple of PAGE_SIZE).
    pub offset: u64,
    pub data: alloc::vec::Vec<u8>,
    pub dirty: bool,
    pub referenced: bool,
}

impl CachePage {
    pub fn new(offset: u64) -> Self {
        Self {
            offset,
            data: alloc::vec![0u8; PAGE_SIZE],
            dirty: false,
            referenced: true,
        }
    }
}

// ── Inode ────────────────────────────────────────────────────────────────────

/// An Inode represents a single file/directory in the VFS.
pub struct Inode {
    pub ino: u64,
    /// Unix mode bits (S_IFREG, S_IFDIR, S_IFLNK, …)
    pub mode: u16,
    pub size: u64,
    pub uid: u32,
    pub gid: u32,
    pub link_count: u32,
    /// Access time (nanoseconds since epoch).
    pub atime_ns: u64,
    /// Modification time (nanoseconds since epoch).
    pub mtime_ns: u64,
    /// Change time (metadata change, nanoseconds since epoch).
    pub ctime_ns: u64,
    /// Page-index → cached page.
    pub pages: Mutex<BTreeMap<u64, Arc<Mutex<CachePage>>>>,
}

impl Inode {
    pub fn new(ino: u64, mode: u16) -> Self {
        Self {
            ino,
            mode,
            size: 0,
            uid: 0,
            gid: 0,
            link_count: 1,
            atime_ns: 0,
            mtime_ns: 0,
            ctime_ns: 0,
            pages: Mutex::new(BTreeMap::new()),
        }
    }

    /// Update access time (called on reads).
    pub fn touch_atime(&mut self, now_ns: u64) {
        self.atime_ns = now_ns;
    }

    /// Update modification and change time (called on writes).
    pub fn touch_mtime(&mut self, now_ns: u64) {
        self.mtime_ns = now_ns;
        self.ctime_ns = now_ns;
    }

    /// Update only change time (metadata changes like chmod/chown).
    pub fn touch_ctime(&mut self, now_ns: u64) {
        self.ctime_ns = now_ns;
    }

    /// Read bytes from the page cache.
    /// Returns 0 on a cache miss; the caller must fetch from the backing store.
    pub fn read_cached(&self, offset: u64, buf: &mut [u8]) -> usize {
        let mut read = 0usize;
        let mut cur = offset;
        let cache = self.pages.lock();

        while read < buf.len() && cur < self.size {
            let idx = cur / PAGE_SIZE as u64;
            let poff = (cur % PAGE_SIZE as u64) as usize;

            let Some(page) = cache.get(&idx) else { break };
            let mut p = page.lock();
            p.referenced = true;

            let avail = (p.data.len() - poff).min(buf.len() - read);
            let clamped = (avail as u64).min(self.size - cur) as usize;
            if clamped == 0 {
                break;
            }

            buf[read..read + clamped].copy_from_slice(&p.data[poff..poff + clamped]);
            read += clamped;
            cur += clamped as u64;
        }
        read
    }

    /// Write bytes into the page cache (creates pages on demand).
    pub fn write_cached(&mut self, offset: u64, data: &[u8]) -> usize {
        let mut written = 0usize;
        let mut cur = offset;
        let mut cache = self.pages.lock();

        while written < data.len() {
            let idx = cur / PAGE_SIZE as u64;
            let poff = (cur % PAGE_SIZE as u64) as usize;

            let page = cache
                .entry(idx)
                .or_insert_with(|| Arc::new(Mutex::new(CachePage::new(idx * PAGE_SIZE as u64))));

            let mut p = page.lock();
            let chunk = (p.data.len() - poff).min(data.len() - written);
            p.data[poff..poff + chunk].copy_from_slice(&data[written..written + chunk]);
            p.dirty = true;
            p.referenced = true;
            // Notify writeback engine about dirty page
            super::writeback::mark_dirty(self.ino, idx);
            written += chunk;
            cur += chunk as u64;
        }
        if cur > self.size {
            self.size = cur;
        }
        written
    }

    /// Flush all dirty pages for this inode to stable storage.
    pub fn fsync(&self) -> Result<usize, &'static str> {
        super::writeback::fsync_inode(self.ino)
    }
}

// ── Dentry ───────────────────────────────────────────────────────────────────

/// A Directory Entry: one knot in the dentry tree used for O(log n) path
/// resolution without touching the backing file-system for every lookup.
pub struct Dentry {
    pub name: String,
    pub inode: Arc<Inode>,
    /// Version tag — incremented on every mutation for cache invalidation.
    pub version: AtomicUsize,
    pub children: Mutex<BTreeMap<String, Arc<Dentry>>>,
}

impl Dentry {
    pub fn new(name: String, inode: Arc<Inode>) -> Self {
        Self {
            name,
            inode,
            version: AtomicUsize::new(0),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    /// Bump the version (on insert/remove child).
    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Current version.
    pub fn get_version(&self) -> usize {
        self.version.load(Ordering::Relaxed)
    }

    /// Look up a direct child by name.
    pub fn child(&self, name: &str) -> Option<Arc<Dentry>> {
        self.children.lock().get(name).cloned()
    }

    /// Insert / overwrite a direct child.
    pub fn insert_child(&self, name: String, dentry: Arc<Dentry>) {
        self.children.lock().insert(name, dentry);
        self.bump_version();
    }

    /// Walk a slash-separated path starting from this dentry.
    /// Returns `Ok(Arc<Dentry>)` on success, `Err` if a component is missing.
    pub fn lookup(&self, path: &str) -> Result<Arc<Dentry>, &'static str> {
        // Strip leading '/' and split.
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            // Shouldn't happen but handle gracefully.
            return Err("empty path");
        }

        let mut components = path.splitn(2, '/');
        let first = components.next().unwrap_or("");
        let rest = components.next().unwrap_or("");

        let child = self.child(first).ok_or("dentry not found")?;

        if rest.is_empty() {
            Ok(child)
        } else {
            child.lookup(rest)
        }
    }

    /// Walk a path, creating missing dentry nodes lazily with the provided
    /// inode factory `make_inode(ino_counter, component_name)`.
    pub fn lookup_or_create<F>(&self, path: &str, make_inode: &mut F) -> Arc<Dentry>
    where
        F: FnMut(&str) -> Arc<Inode>,
    {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            // Return ourselves; caller passed the root.
            return Arc::new(Dentry::new(self.name.clone(), self.inode.clone()));
        }

        let mut components = path.splitn(2, '/');
        let first = components.next().unwrap();
        let rest = components.next().unwrap_or("");

        // Use or insert the child for `first`.
        let child = {
            let mut ch = self.children.lock();
            if let Some(existing) = ch.get(first) {
                existing.clone()
            } else {
                let inode = make_inode(first);
                let dentry = Arc::new(Dentry::new(first.to_string(), inode));
                ch.insert(first.to_string(), dentry.clone());
                dentry
            }
        };

        if rest.is_empty() {
            child
        } else {
            child.lookup_or_create(rest, make_inode)
        }
    }
}

// ── Inode Cache ──────────────────────────────────────────────────────────────

/// Global inode number counter (monotonically increasing).
static GLOBAL_INO_COUNTER: AtomicUsize = AtomicUsize::new(2); // 1 is reserved for root

pub fn alloc_ino() -> u64 {
    GLOBAL_INO_COUNTER.fetch_add(1, Ordering::Relaxed) as u64
}

/// System-wide flat inode cache: ino → Arc<Inode>.
pub struct InodeCache {
    inodes: Mutex<BTreeMap<u64, Arc<Inode>>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl InodeCache {
    pub const fn new() -> Self {
        Self {
            inodes: Mutex::new(BTreeMap::new()),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, ino: u64) -> Option<Arc<Inode>> {
        let t = self.inodes.lock();
        match t.get(&ino) {
            Some(i) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(i.clone())
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    pub fn insert(&self, inode: Arc<Inode>) {
        self.inodes.lock().insert(inode.ino, inode);
    }

    pub fn evict(&self, ino: u64) {
        self.inodes.lock().remove(&ino);
    }

    /// Returns (hits, misses).
    pub fn stats(&self) -> (usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

/// Global flat inode cache.
pub static GLOBAL_INODE_CACHE: InodeCache = InodeCache::new();

// ── CachedFileSystem ─────────────────────────────────────────────────────────

/// Wraps any `FileSystem` and layers a dentry cache in front of it.
/// Repeated opens of the same path are resolved by the dentry tree in O(log n)
/// without calling through to the inner FS.
pub struct CachedFileSystem<FS: crate::modules::vfs::FileSystem> {
    inner: FS,
    /// Root of the dentry tree for this mount.
    root: Arc<Dentry>,
    /// Number of dentry cache hits (for telemetry).
    dentry_hits: AtomicUsize,
    /// Number of dentry cache misses (inner FS was called).
    dentry_misses: AtomicUsize,
}

impl<FS: crate::modules::vfs::FileSystem> CachedFileSystem<FS> {
    pub fn new(inner: FS) -> Self {
        let root_inode = Arc::new(Inode::new(1, 0o040755)); // directory
        GLOBAL_INODE_CACHE.insert(root_inode.clone());
        Self {
            inner,
            root: Arc::new(Dentry::new("/".to_string(), root_inode)),
            dentry_hits: AtomicUsize::new(0),
            dentry_misses: AtomicUsize::new(0),
        }
    }

    pub fn dentry_stats(&self) -> (usize, usize) {
        (
            self.dentry_hits.load(Ordering::Relaxed),
            self.dentry_misses.load(Ordering::Relaxed),
        )
    }

    /// Resolve `path` through the dentry tree; on a miss, create a new dentry
    /// backed by a fresh inode and record the hit for future lookups.
    fn resolve_dentry(&self, path: &str) -> Arc<Dentry> {
        // Quick check: is the whole path already in the tree?
        if let Ok(existing) = self.root.lookup(path) {
            self.dentry_hits.fetch_add(1, Ordering::Relaxed);
            return existing;
        }

        // Cache miss: create dentries lazily.
        self.dentry_misses.fetch_add(1, Ordering::Relaxed);
        self.root.lookup_or_create(path, &mut |_name| {
            let ino = alloc_ino();
            let inode = Arc::new(Inode::new(ino, 0o100644)); // regular file
            GLOBAL_INODE_CACHE.insert(inode.clone());
            inode
        })
    }
}

impl<FS: crate::modules::vfs::FileSystem> crate::modules::vfs::FileSystem for CachedFileSystem<FS> {
    fn open(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<alloc::boxed::Box<dyn crate::modules::vfs::File>, &'static str> {
        // Warm up the dentry cache (creates entries if missing).
        let _dentry = self.resolve_dentry(path);
        // Always delegate real I/O to the inner FS.
        self.inner.open(path, tid)
    }

    fn create(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<alloc::boxed::Box<dyn crate::modules::vfs::File>, &'static str> {
        // Create the dentry eagerly so subsequent opens are cached.
        let _ = self.resolve_dentry(path);
        self.inner.create(path, tid)
    }

    fn remove(&self, path: &str, tid: crate::interfaces::TaskId) -> Result<(), &'static str> {
        // Evict the dentry from the tree on removal.
        // Simple approach: just evict from the flat inode cache.
        if let Ok(d) = self.root.lookup(path) {
            GLOBAL_INODE_CACHE.evict(d.inode.ino);
        }
        self.inner.remove(path, tid)
    }

    fn mkdir(&self, path: &str, tid: crate::interfaces::TaskId) -> Result<(), &'static str> {
        self.inner.mkdir(path, tid)
    }

    fn rmdir(&self, path: &str, tid: crate::interfaces::TaskId) -> Result<(), &'static str> {
        self.inner.rmdir(path, tid)
    }

    fn readdir(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<alloc::vec::Vec<crate::modules::vfs::types::DirEntry>, &'static str> {
        self.inner.readdir(path, tid)
    }

    fn stat(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        self.inner.stat(path, tid)
    }
}

#[path = "cache/negative.rs"]
mod negative;

pub use negative::{mark_block_journaled, NegativeDentryCache, NEGATIVE_DENTRY_CACHE};
