use super::types::{FileSystem, File, FileStats, FileType};
use super::cache::{CachePage, Inode};
use crate::interfaces::TaskId;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

/// Aether-FS: The native, high-performance filesystem for AetherXOS.
/// Optimized for Zero-Copy Page Cache and hardware-aligned I/O.
pub struct AetherFS {
    pub root_inode: Arc<Inode>,
}

impl FileSystem for AetherFS {
    fn open(&self, _path: &str, _tid: TaskId) -> Result<alloc::boxed::Box<dyn File>, &'static str> {
        // High-performance path: resolve directly to an AetherFile
        Ok(alloc::boxed::Box::new(AetherFile {
            inode: self.root_inode.clone(), // Mock: always return root for now
            offset: 0,
        }))
    }

    fn stat(&self, _path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            ino: 1,
            size: 4096,
            mode: 0o755,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            blocks: 1,
            atime: (0, 0),
            mtime: (0, 0),
            ctime: (0, 0),
        })
    }

    fn create(&self, _path: &str, _tid: TaskId) -> Result<alloc::boxed::Box<dyn File>, &'static str> {
        let ino = super::cache::alloc_ino();
        let inode = Arc::new(Inode::new(ino, 0o100644)); // S_IFREG | 0644
        super::cache::GLOBAL_INODE_CACHE.insert(inode.clone());
        Ok(alloc::boxed::Box::new(AetherFile {
            inode,
            offset: 0,
        }))
    }

    fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Ok(())
    }

    fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Ok(())
    }
}

pub struct AetherFile {
    pub inode: Arc<Inode>,
    pub offset: u64,
}

impl File for AetherFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        // Native Zero-Copy Read: Transfer directly from Page Cache
        let read = self.inode.read_cached(self.offset, buf);
        self.offset += read as u64;
        Ok(read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let written = self.inode.write_cached(self.offset, buf);
        self.offset += written as u64;
        Ok(written)
    }

    fn seek(&mut self, pos: super::SeekFrom) -> Result<u64, &'static str> {
        match pos {
            super::SeekFrom::Start(s) => self.offset = s,
            super::SeekFrom::Current(c) => self.offset = (self.offset as i64 + c) as u64,
            super::SeekFrom::End(e) => self.offset = (self.inode.size as i64 + e) as u64,
        }
        Ok(self.offset)
    }

    fn mmap_physical(&self, offset: u64, len: usize) -> Result<Vec<u64>, &'static str> {
        let mut frames = Vec::new();
        let mut cur = offset;
        let end = offset + len as u64;
        let mut pages = self.inode.pages.lock();

        while cur < end {
            let idx = cur / 4096;
            let page = pages.entry(idx).or_insert_with(|| Arc::new(Mutex::new(CachePage::new(idx * 4096))));
            frames.push(page.lock().phys_addr);
            cur += 4096;
        }
        Ok(frames)
    }

    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
