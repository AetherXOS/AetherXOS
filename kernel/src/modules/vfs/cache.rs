use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use aethercore_common::units::PAGE_SIZE_4K;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::{Mutex, RwLock};

// ── Page Cache ────────────────────────────────────────────────────────────────

/// The kernel page size used by the VFS page cache.
const PAGE_SIZE: usize = PAGE_SIZE_4K;

/// One 4 KiB page in the VFS Page Cache, backed by a physical frame.
pub struct CachePage {
    /// Byte offset within the file (always a multiple of PAGE_SIZE).
    pub offset: u64,
    /// Physical address of the frame.
    pub phys_addr: u64,
    #[cfg(not(target_os = "none"))]
    pub host_data: Option<alloc::boxed::Box<[u8; PAGE_SIZE]>>,
    pub dirty: bool,
    pub referenced: bool,
}

impl CachePage {
    pub fn new(offset: u64) -> Self {
        #[cfg(target_os = "none")]
        {
            #[cfg(feature = "paging_enable")]
            let phys_addr = {
                let mut alloc = crate::kernel::vmm::GLOBAL_PAGE_ALLOC.lock();
                alloc.allocate_pages(0).expect("OOM in VFS CachePage") as u64
            };
            #[cfg(not(feature = "paging_enable"))]
            let phys_addr = 0u64;
            
            Self {
                offset,
                phys_addr,
                dirty: false,
                referenced: true,
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            Self {
                offset,
                phys_addr: 0,
                host_data: Some(alloc::boxed::Box::new([0u8; PAGE_SIZE])),
                dirty: false,
                referenced: true,
            }
        }
    }

    /// Get a kernel-virtual pointer to the page data.
    pub fn data_ptr(&self) -> *mut u8 {
        #[cfg(target_os = "none")]
        {
            let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
            (self.phys_addr + hhdm) as *mut u8
        }
        #[cfg(not(target_os = "none"))]
        {
            match self.host_data.as_ref() {
                Some(data) => data.as_ptr() as *mut u8,
                None => core::ptr::null_mut(),
            }
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data_ptr(), PAGE_SIZE) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.data_ptr(), PAGE_SIZE) }
    }
}

impl Drop for CachePage {
    fn drop(&mut self) {
        #[cfg(target_os = "none")]
        #[cfg(feature = "paging_enable")]
        {
            let mut alloc = crate::kernel::vmm::GLOBAL_PAGE_ALLOC.lock();
            alloc.deallocate_pages(self.phys_addr as usize, 0);
        }
    }
}

// ── Inode ────────────────────────────────────────────────────────────────────

/// An Inode represents a single file/directory in the VFS.
pub struct Inode {
    pub ino: u64,
    /// Unix mode bits (S_IFREG, S_IFDIR, S_IFLNK, …)
    pub mode: u16,
    pub size: core::sync::atomic::AtomicU64,
    pub uid: core::sync::atomic::AtomicU32,
    pub gid: core::sync::atomic::AtomicU32,
    pub nlink: core::sync::atomic::AtomicU32,
    pub link_count: core::sync::atomic::AtomicU32,
    /// Access time (nanoseconds since epoch).
    pub atime_ns: core::sync::atomic::AtomicU64,
    /// Modification time (nanoseconds since epoch).
    pub mtime_ns: core::sync::atomic::AtomicU64,
    /// Change time (metadata change, nanoseconds since epoch).
    pub ctime_ns: core::sync::atomic::AtomicU64,
    /// Shards of page maps to minimize lock contention.
    pub pages: [Mutex<BTreeMap<u64, Arc<Mutex<CachePage>>>>; crate::config::vfs::PAGE_CACHE_SHARD_COUNT],
}

impl Inode {
    pub fn new(ino: u64, mode: u16) -> Self {
        const SHARD_INIT: Mutex<BTreeMap<u64, Arc<Mutex<CachePage>>>> = Mutex::new(BTreeMap::new());
        Self {
            ino,
            mode,
            size: core::sync::atomic::AtomicU64::new(0),
            uid: core::sync::atomic::AtomicU32::new(0),
            gid: core::sync::atomic::AtomicU32::new(0),
            nlink: core::sync::atomic::AtomicU32::new(1),
            link_count: core::sync::atomic::AtomicU32::new(1),
            atime_ns: core::sync::atomic::AtomicU64::new(0),
            mtime_ns: core::sync::atomic::AtomicU64::new(0),
            ctime_ns: core::sync::atomic::AtomicU64::new(0),
            pages: [SHARD_INIT; crate::config::vfs::PAGE_CACHE_SHARD_COUNT],
        }
    }

    pub fn get_page_shard(&self, idx: u64) -> usize {
        idx as usize % crate::config::vfs::PAGE_CACHE_SHARD_COUNT 
    }

    /// Update access time (called on reads).
    pub fn touch_atime(&self, now_ns: u64) {
        self.atime_ns.store(now_ns, Ordering::Relaxed);
    }

    /// Update modification and change time (called on writes).
    pub fn touch_mtime(&self, now_ns: u64) {
        self.mtime_ns.store(now_ns, Ordering::Relaxed);
        self.ctime_ns.store(now_ns, Ordering::Relaxed);
    }

    /// Update only change time (metadata changes like chmod/chown).
    pub fn touch_ctime(&self, now_ns: u64) {
        self.ctime_ns.store(now_ns, Ordering::Relaxed);
    }

    /// Read bytes from the page cache.
    /// Returns 0 on a cache miss; the caller must fetch from the backing store.
    pub fn read_cached(&self, offset: u64, buf: &mut [u8]) -> usize {
        let mut read = 0usize;
        let mut cur = offset;
        let size = self.size.load(Ordering::Relaxed);

        while read < buf.len() && cur < size {
            let idx = cur / PAGE_SIZE as u64;
            let poff = (cur % PAGE_SIZE as u64) as usize;
            let shard = self.get_page_shard(idx);

            let cache = self.pages[shard].lock();
            let Some(page) = cache.get(&idx) else { break };
            let mut p = page.lock();
            p.referenced = true;

            let avail = (PAGE_SIZE - poff).min(buf.len() - read);
            let clamped = (avail as u64).min(self.size.load(core::sync::atomic::Ordering::Relaxed) - cur) as usize;
            if clamped == 0 {
                break;
            }

            buf[read..read + clamped].copy_from_slice(&p.as_slice()[poff..poff + clamped]);
            read += clamped;
            cur += clamped as u64;

            // Trigger Predictive Prefetching (PPP) for the next X pages
            self.prefetch(idx + 1, crate::config::vfs::PREDICTIVE_PREFETCH_COUNT as u64);
        }
        read
    }

    /// Predictive Page Prefetcher (PPP).
    pub fn prefetch(&self, start_idx: u64, count: u64) {
        let size = self.size.load(Ordering::Relaxed);
        for i in 0..count {
            let idx = start_idx + i;
            if idx * PAGE_SIZE as u64 >= size { break; }
            let shard = self.get_page_shard(idx);
            
            let mut cache = self.pages[shard].lock();
            cache.entry(idx).or_insert_with(|| {
                Arc::new(Mutex::new(CachePage::new(idx * PAGE_SIZE as u64)))
            });
        }
    }

    /// Write bytes into the page cache (creates pages on demand).
    pub fn write_cached(&self, offset: u64, data: &[u8]) -> usize {
        let mut written = 0usize;
        let mut cur = offset;

        while written < data.len() {
            let idx = cur / PAGE_SIZE as u64;
            let poff = (cur % PAGE_SIZE as u64) as usize;
            let shard = self.get_page_shard(idx);

            let mut cache = self.pages[shard].lock();
            let page = cache
                .entry(idx)
                .or_insert_with(|| Arc::new(Mutex::new(CachePage::new(idx * PAGE_SIZE as u64))));

            let mut p = page.lock();
            let chunk = (PAGE_SIZE - poff).min(data.len() - written);
            p.as_slice_mut()[poff..poff + chunk].copy_from_slice(&data[written..written + chunk]);
            p.dirty = true;
            p.referenced = true;
            // Notify writeback engine about dirty page
            super::writeback::mark_dirty(self.ino, idx);
            written += chunk;
            cur += chunk as u64;
        }
        
        let mut old_size = self.size.load(Ordering::Relaxed);
        while cur > old_size {
            match self.size.compare_exchange_weak(old_size, cur, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(new_val) => old_size = new_val,
            }
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
/// Optimized with a Sharded-Lock architecture for maximum concurrency.
pub struct Dentry {
    pub name: String,
    pub inode: Arc<Inode>,
    /// Version tag — incremented on every mutation for cache invalidation.
    pub version: AtomicUsize,
    /// 16 shards of child maps to minimize lock contention on high-core systems.
    pub children: [RwLock<BTreeMap<String, Arc<Dentry>>>; 16],
}

impl Dentry {
    pub fn new(name: String, inode: Arc<Inode>) -> Self {
        const SHARD_INIT: RwLock<BTreeMap<String, Arc<Dentry>>> = RwLock::new(BTreeMap::new());
        Self {
            name,
            inode,
            version: AtomicUsize::new(0),
            children: [SHARD_INIT; 16],
        }
    }

    fn get_shard_idx(&self, name: &str) -> usize {
        let mut h = 0usize;
        for b in name.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(*b as usize);
        }
        h % 16
    }

    /// Bump the version (on insert/remove child).
    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Current version.
    pub fn get_version(&self) -> usize {
        self.version.load(Ordering::Relaxed)
    }

    pub fn child(&self, name: &str) -> Option<Arc<Dentry>> {
        let idx = self.get_shard_idx(name);
        self.children[idx].read().get(name).cloned()
    }

    /// Insert / overwrite a direct child (Lock-Contention Minimized).
    pub fn insert_child(&self, name: String, dentry: Arc<Dentry>) {
        let idx = self.get_shard_idx(&name);
        self.children[idx].write().insert(name, dentry);
        self.bump_version();
    }

    /// Walk a slash-separated path starting from this dentry.
    /// Returns `Ok(Arc<Dentry>)` on success, `Err` if a component is missing.
    pub fn lookup(&self, path: &str) -> Result<Arc<Dentry>, &'static str> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Ok(Arc::new(Dentry::new(self.name.clone(), self.inode.clone())));
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
            return Arc::new(Dentry::new(self.name.clone(), self.inode.clone()));
        }

        let mut components = path.splitn(2, '/');
        let first = components.next().unwrap_or("");
        let rest = components.next().unwrap_or("");

        // Use or insert the child for `first`.
        let child = {
            let idx = self.get_shard_idx(first);
            let mut ch = self.children[idx].write();
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

    /// Iterate over all cached inodes and call `f` for each one.
    pub fn for_each<F: FnMut(u64, &Arc<Inode>)>(&self, mut f: F) {
        let map = self.inodes.lock();
        for (&ino, inode) in map.iter() {
            f(ino, inode);
        }
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

    /// Resolve `path` through the dentry tree; on a miss, fetch real metadata
    /// from the inner FS and record it in the cache.
    pub fn resolve_dentry_with_stats(&self, path: &str, stats: &crate::modules::vfs::types::FileStats) -> Arc<Dentry> {
        // Quick check: is the whole path already in the tree?
        if let Ok(existing) = self.root.lookup(path) {
            self.dentry_hits.fetch_add(1, Ordering::Relaxed);
            // Optional: update cached inode metadata if stats differ significantly
            return existing;
        }

        // Cache miss: create dentries with real metadata.
        self.dentry_misses.fetch_add(1, Ordering::Relaxed);
        self.root.lookup_or_create(path, &mut |_name| {
            let inode_arc = Arc::new(Inode::new(stats.ino, stats.mode as u16));
            inode_arc.size.store(stats.size, Ordering::Relaxed);
            inode_arc.uid.store(stats.uid, Ordering::Relaxed);
            inode_arc.gid.store(stats.gid, Ordering::Relaxed);
            inode_arc.nlink.store(stats.nlink, Ordering::Relaxed);
            inode_arc.atime_ns.store(stats.atime.sec * 1_000_000_000 + stats.atime.nsec as u64, Ordering::Relaxed);
            inode_arc.mtime_ns.store(stats.mtime.sec * 1_000_000_000 + stats.mtime.nsec as u64, Ordering::Relaxed);
            inode_arc.ctime_ns.store(stats.ctime.sec * 1_000_000_000 + stats.ctime.nsec as u64, Ordering::Relaxed);
            
            GLOBAL_INODE_CACHE.insert(inode_arc.clone());
            inode_arc
        })
    }
}

pub struct CachedFile {
    pub inner: alloc::boxed::Box<dyn crate::modules::vfs::File>,
    pub inode: Arc<Inode>,
    pub offset: u64,
}

impl crate::modules::vfs::File for CachedFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        // Try cache first
        let read = self.inode.read_cached(self.offset, buf);
        if read > 0 {
            self.offset += read as u64;
            // Also sync inner offset if it supports seeking
            let _ = self.inner.seek(crate::modules::vfs::SeekFrom::Start(self.offset));
            return Ok(read);
        }
        
        // Cache miss: read from inner
        let read = self.inner.read(buf)?;
        if read > 0 {
            // Populate cache for future reads
            self.inode.write_cached(self.offset, &buf[..read]);
            self.offset += read as u64;
        }
        Ok(read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        self.inner.write(buf)
    }

    fn seek(&mut self, pos: crate::modules::vfs::SeekFrom) -> Result<u64, &'static str> {
        let size = self.inode.size.load(Ordering::Relaxed);
        let new_off = match pos {
            crate::modules::vfs::SeekFrom::Start(off) => off,
            crate::modules::vfs::SeekFrom::Current(off) => (self.offset as i64 + off) as u64,
            crate::modules::vfs::SeekFrom::End(off) => (size as i64 + off) as u64,
        };
        self.offset = new_off;
        self.inner.seek(pos)
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        self.inner.flush()
    }

    fn stat(&self) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        self.inner.stat()
    }

    fn mmap_physical(&self, offset: u64, len: usize) -> Result<alloc::vec::Vec<u64>, &'static str> {
        let mut frames = alloc::vec::Vec::new();
        let mut cur = offset;
        let end = offset + len as u64;
        while cur < end {
            let idx = cur / PAGE_SIZE as u64;
            let shard = self.inode.get_page_shard(idx);
            let mut pages = self.inode.pages[shard].lock();
            let page = pages.entry(idx).or_insert_with(|| Arc::new(Mutex::new(CachePage::new(idx * PAGE_SIZE as u64))));
            frames.push(page.lock().phys_addr);
            cur += PAGE_SIZE as u64;
        }
        Ok(frames)
    }

    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }

    fn try_clone(&self) -> Result<alloc::boxed::Box<dyn crate::modules::vfs::File>, &'static str> {
        Ok(alloc::boxed::Box::new(CachedFile {
            inner: self.inner.try_clone()?,
            inode: self.inode.clone(),
            offset: self.offset,
        }))
    }
}

impl<FS: crate::modules::vfs::FileSystem> crate::modules::vfs::FileSystem for CachedFileSystem<FS> {
    fn open(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<alloc::boxed::Box<dyn crate::modules::vfs::File>, &'static str> {
        // Warm up the dentry cache (creates entries if missing).
        // For real high-fidelity, we should fetch stats first.
        let stats = self.inner.stat(path, tid)?;
        let dentry = self.resolve_dentry_with_stats(path, &stats);
        
        let inner_file = self.inner.open(path, tid)?;
        Ok(alloc::boxed::Box::new(CachedFile {
            inner: inner_file,
            inode: dentry.inode.clone(),
            offset: 0,
        }))
    }

    fn create(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<alloc::boxed::Box<dyn crate::modules::vfs::File>, &'static str> {
        let inner_file = self.inner.create(path, tid)?;
        let stats = inner_file.stat()?;
        let dentry = self.resolve_dentry_with_stats(path, &stats);
        
        Ok(alloc::boxed::Box::new(CachedFile {
            inner: inner_file,
            inode: dentry.inode.clone(),
            offset: 0,
        }))
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

    fn statfs(
        &self,
        path: &str,
        tid: crate::interfaces::TaskId,
    ) -> Result<crate::modules::vfs::types::FsStats, &'static str> {
        self.inner.statfs(path, tid)
    }

    fn lookup_dentry(&self, path: &str) -> Option<Arc<Dentry>> {
        self.root.lookup(path).ok()
    }
}

#[path = "cache/negative.rs"]
mod negative;

pub use negative::{mark_block_journaled, NegativeDentryCache, NEGATIVE_DENTRY_CACHE};
