//! Writable Filesystem Layer
//!
//! Implements a writable overlay on top of read-only filesystem backends (like ext4-view).
//! Uses a copy-on-write strategy:
//! - Reads go to the overlay first, then fall through to the read-only base.
//! - Writes go to a RAM-backed overlay with dirty page tracking.
//! - fsync/sync flushes dirty pages through the writeback engine to the block device.
//! - A write-ahead journal ensures crash consistency for metadata operations.
//!
//! This approach lets us support writable ext4/FAT/SquashFS without modifying the
//! read-only crate internals, while still achieving persistence via the block driver.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::interfaces::TaskId;
use crate::modules::vfs::cache::{alloc_ino, CachePage, Inode, GLOBAL_INODE_CACHE};
use crate::modules::vfs::types::{DirEntry, File, FileStats, FileSystem, SeekFrom};
use crate::modules::vfs::writable_fs_support::{simple_hash, OverlayEntry};
use crate::modules::vfs::writeback::{self, JournalOp, JournalTransaction, WritebackSink};
#[path = "writable_fs/block_sink.rs"]
mod block_sink;
#[path = "writable_fs/filesystem_impl.rs"]
mod filesystem_impl;
#[path = "writable_fs/ram_sink.rs"]
mod ram_sink;
pub use block_sink::{BlockDeviceAdapter, BlockWritebackSink};
pub use ram_sink::RamWritebackSink;

const PAGE_SIZE: usize = 4096;
const DT_REG: u8 = 8;
const DT_DIR: u8 = 4;
const DT_LNK: u8 = 10;
// ÔöÇÔöÇ Block-Backed Writeback Sink ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

// ÔöÇÔöÇ RAM-backed Writeback Sink (for testing / RamFS persistence) ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

// ÔöÇÔöÇ Writable Overlay Filesystem ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

/// A copy-on-write overlay filesystem.
///
/// - Read path: check overlay first ÔåÆ fall through to base FS.
/// - Write path: copy-up to overlay ÔåÆ write to page cache ÔåÆ mark dirty.
/// - Deleted files: recorded as whiteouts so they don't show from base.
/// - fsync: flushes dirty pages via the writeback engine.
pub struct WritableOverlayFs<Base: FileSystem> {
    /// Read-only base filesystem.
    base: Base,
    /// Overlay metadata: path ÔåÆ entry.
    entries: Mutex<BTreeMap<String, OverlayEntry>>,
    /// Directory children tracking: parent_path ÔåÆ set of child names.
    dir_children: Mutex<BTreeMap<String, Vec<String>>>,
    /// Path ÔåÆ inode mapping for the overlay.
    path_to_ino: Mutex<BTreeMap<String, u64>>,
    /// Mount ID for writeback registration.
    #[allow(dead_code)]
    mount_id: usize,
}

impl<Base: FileSystem> WritableOverlayFs<Base> {
    /// Create a new writable overlay on top of a read-only base filesystem.
    /// Registers with the writeback engine using the provided sink.
    pub fn new(base: Base, mount_id: usize, sink: Arc<dyn WritebackSink>) -> Self {
        writeback::register_writable_mount(mount_id, sink);

        // Create root directory entry
        let root_ino = alloc_ino();
        let root_entry = OverlayEntry::new_dir(root_ino, 0o755);

        let mut entries = BTreeMap::new();
        entries.insert("/".to_string(), root_entry);

        let mut path_to_ino = BTreeMap::new();
        path_to_ino.insert("/".to_string(), root_ino);

        Self {
            base,
            entries: Mutex::new(entries),
            dir_children: Mutex::new(BTreeMap::new()),
            path_to_ino: Mutex::new(path_to_ino),
            mount_id,
        }
    }

    fn normalize(path: &str) -> String {
        let mut p = path.trim_start_matches('/').to_string();
        if p.is_empty() {
            return "/".to_string();
        }
        // Remove trailing slash for files
        while p.ends_with('/') && p.len() > 1 {
            p.pop();
        }
        alloc::format!("/{}", p)
    }

    fn parent_path(path: &str) -> Option<String> {
        let norm = Self::normalize(path);
        if norm == "/" {
            return None;
        }
        if let Some(pos) = norm.rfind('/') {
            if pos == 0 {
                Some("/".to_string())
            } else {
                Some(norm[..pos].to_string())
            }
        } else {
            Some("/".to_string())
        }
    }

    fn basename(path: &str) -> &str {
        let norm = path.trim_end_matches('/');
        if let Some(pos) = norm.rfind('/') {
            &norm[pos + 1..]
        } else {
            norm
        }
    }

    /// Check if a path exists in the overlay (not whited-out).
    fn overlay_exists(&self, path: &str) -> bool {
        let entries = self.entries.lock();
        if let Some(entry) = entries.get(path) {
            !entry.whiteout
        } else {
            false
        }
    }

    /// Check if a path is whited-out (deleted in overlay).
    fn is_whiteout(&self, path: &str) -> bool {
        let entries = self.entries.lock();
        entries.get(path).map_or(false, |e| e.whiteout)
    }

    /// Copy-up: copy a file from the base FS to the overlay for modification.
    fn copy_up(&self, path: &str, tid: TaskId) -> Result<u64, &'static str> {
        let norm = Self::normalize(path);
        // Already in overlay?
        if self.overlay_exists(&norm) {
            let entries = self.entries.lock();
            return Ok(entries.get(&norm).unwrap().ino);
        }

        // Read from base
        let base_stat = self.base.stat(path, tid)?;
        let ino = alloc_ino();

        let mut entry = OverlayEntry::new_file(ino, base_stat.mode as u16);
        entry.size = base_stat.size;
        entry.uid = base_stat.uid;
        entry.gid = base_stat.gid;
        entry.atime = base_stat.atime;
        entry.mtime = base_stat.mtime;
        entry.ctime = base_stat.ctime;

        // Read base file content into page cache
        let inode = Arc::new(Inode::new(ino, entry.mode));
        GLOBAL_INODE_CACHE.insert(inode.clone());
        writeback::register_inode(ino, self.mount_id);

        // Copy content from base
        if let Ok(mut base_file) = self.base.open(path, tid) {
            let mut offset = 0u64;
            let mut buf = [0u8; PAGE_SIZE];
            loop {
                match base_file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Write to inode's page cache (but don't mark dirty ÔÇö this is copy-up, not a user write)
                        let idx = offset / PAGE_SIZE as u64;
                        let mut cache = inode.pages.lock();
                        let page = cache.entry(idx).or_insert_with(|| {
                            Arc::new(spin::Mutex::new(CachePage::new(idx * PAGE_SIZE as u64)))
                        });
                        let mut p = page.lock();
                        let copy_len = n.min(p.data.len());
                        p.data[..copy_len].copy_from_slice(&buf[..copy_len]);
                        p.referenced = true;
                        // NOT marking dirty here ÔÇö the data matches the base
                        offset += n as u64;
                    }
                    Err(_) => break,
                }
            }
        }

        // Update inode size
        {
            // Access through Arc ÔÇö Inode fields are not behind Mutex except `pages`
            // so we need to handle this carefully. In our design, size is set during
            // write_cached, but for copy-up we set it on the entry.
        }

        self.entries.lock().insert(norm.clone(), entry);
        self.path_to_ino.lock().insert(norm, ino);
        Ok(ino)
    }

    /// Get or create the inode for a path in the overlay.
    fn get_or_create_inode(&self, path: &str) -> Option<u64> {
        let norm = Self::normalize(path);
        self.path_to_ino.lock().get(&norm).copied()
    }
}

// ÔöÇÔöÇ Overlay File Handle ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

/// An open file handle in the writable overlay.
struct OverlayFile {
    ino: u64,
    inode: Arc<Inode>,
    cursor: u64,
    size: u64,
    #[allow(dead_code)]
    mount_id: usize,
}

impl File for OverlayFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let n = self.inode.read_cached(self.cursor, buf);
        self.cursor += n as u64;
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        // We need mutable access to the inode for write_cached.
        // Since Inode::write_cached takes &mut self, but we hold Arc<Inode>,
        // we use the page cache directly.
        let mut written = 0usize;
        let mut cur = self.cursor;

        let mut cache = self.inode.pages.lock();
        while written < buf.len() {
            let idx = cur / PAGE_SIZE as u64;
            let poff = (cur % PAGE_SIZE as u64) as usize;

            let page = cache.entry(idx).or_insert_with(|| {
                Arc::new(spin::Mutex::new(CachePage::new(idx * PAGE_SIZE as u64)))
            });

            let mut p = page.lock();
            let chunk = (p.data.len() - poff).min(buf.len() - written);
            p.data[poff..poff + chunk].copy_from_slice(&buf[written..written + chunk]);
            p.dirty = true;
            p.referenced = true;
            writeback::mark_dirty(self.ino, idx);
            written += chunk;
            cur += chunk as u64;
        }
        drop(cache);

        self.cursor = cur;
        if cur > self.size {
            self.size = cur;
        }
        Ok(written)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(delta) => {
                let base = self.size as i128;
                let target = base + delta as i128;
                if target < 0 {
                    return Err("seek before start");
                }
                target as u64
            }
            SeekFrom::Current(delta) => {
                let base = self.cursor as i128;
                let target = base + delta as i128;
                if target < 0 {
                    return Err("seek before start");
                }
                target as u64
            }
        };
        self.cursor = new_pos;
        Ok(new_pos)
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn fsync(&mut self) -> Result<(), &'static str> {
        writeback::fsync_inode(self.ino)?;
        Ok(())
    }

    fn fdatasync(&mut self) -> Result<(), &'static str> {
        // Same as fsync for now (metadata is in-memory anyway)
        self.fsync()
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        self.size = size;
        // Remove pages beyond new size and mark remaining as dirty
        let last_page = if size > 0 {
            (size - 1) / PAGE_SIZE as u64
        } else {
            0
        };
        let mut cache = self.inode.pages.lock();

        // Remove pages beyond the new size
        let remove_keys: Vec<u64> = cache
            .keys()
            .filter(|&&idx| size == 0 || idx > last_page)
            .copied()
            .collect();
        for key in remove_keys {
            cache.remove(&key);
        }

        // Zero-fill the last page if needed
        if size > 0 {
            if let Some(page_arc) = cache.get(&last_page) {
                let mut page = page_arc.lock();
                let keep = (size % PAGE_SIZE as u64) as usize;
                if keep > 0 && keep < PAGE_SIZE {
                    for b in &mut page.data[keep..] {
                        *b = 0;
                    }
                    page.dirty = true;
                    writeback::mark_dirty(self.ino, last_page);
                }
            }
        }

        Ok(())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: self.size,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: PAGE_SIZE as u32,
            blocks: (self.size + 511) / 512,
        })
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

// ÔöÇÔöÇ FileSystem implementation ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

struct RamJournalSink;
impl WritebackSink for RamJournalSink {
    fn write_page(&self, _ino: u64, _offset: u64, _data: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
    fn flush(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "writable_fs/tests.rs"]
mod tests;

