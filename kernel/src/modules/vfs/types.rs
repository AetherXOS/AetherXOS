use crate::interfaces::TaskId;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::any::Any;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PollEvents: u32 {
        const IN = 0x01;
        const OUT = 0x04;
        const ERR = 0x08;
        const HUP = 0x10;
        const PRI = 0x02;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPolicy {
    Buffered,
    Unbuffered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStats {
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub blksize: u32,
    pub blocks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: alloc::string::String,
    pub ino: u64,
    pub kind: u8, // DT_REG, DT_DIR, etc.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Shared,
    Exclusive,
    Unlock,
}

#[derive(Debug, Clone, Copy)]
pub struct IoVec<'a> {
    pub buf: &'a [u8],
}

#[derive(Debug)]
pub struct IoVecMut<'a> {
    pub buf: &'a mut [u8],
}

pub trait File: Send + Sync {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str>;

    fn read_vectored(&mut self, bufs: &mut [IoVecMut]) -> Result<usize, &'static str> {
        let mut total = 0;
        for iov in bufs {
            match self.read(iov.buf) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    if n < iov.buf.len() {
                        break;
                    }
                }
                Err(e) => return if total > 0 { Ok(total) } else { Err(e) },
            }
        }
        Ok(total)
    }

    fn write_vectored(&mut self, bufs: &[IoVec]) -> Result<usize, &'static str> {
        let mut total = 0;
        for iov in bufs {
            match self.write(iov.buf) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    if n < iov.buf.len() {
                        break;
                    }
                }
                Err(e) => return if total > 0 { Ok(total) } else { Err(e) },
            }
        }
        Ok(total)
    }
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Err("seek not supported")
    }
    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    /// Sync file data + metadata to backing store (like fsync(2)).
    fn fsync(&mut self) -> Result<(), &'static str> {
        self.flush()
    }
    /// Sync file data only, no metadata (like fdatasync(2)).
    fn fdatasync(&mut self) -> Result<(), &'static str> {
        self.fsync()
    }
    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Err("truncate not supported")
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Err("stat not supported")
    }

    // Locking
    fn lock(&self, lock_type: LockType) -> Result<(), &'static str> {
        // Default: No-op for backends that don't support locking
        if lock_type == LockType::Unlock {
            Ok(())
        } else {
            Err("locking not supported")
        }
    }

    /// Optional: Return a WaitQueue for poll/select support.
    fn wait_queue(&self) -> Option<&crate::kernel::sync::WaitQueue> {
        None
    }

    // Poll support
    fn poll_events(&self) -> PollEvents {
        // Default to readable/writable for normal files
        PollEvents::IN | PollEvents::OUT
    }

    fn ioctl(&mut self, _cmd: u32, _arg: u64) -> Result<isize, &'static str> {
        Err("ioctl not supported")
    }

    fn mmap(
        &self,
        _offset: u64,
        _len: usize,
    ) -> Result<Arc<Mutex<alloc::vec::Vec<u8>>>, &'static str> {
        Err("mmap not supported")
    }

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait FileSystem: Send + Sync {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str>;
    fn create(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str>;
    fn remove(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;

    // Directory Management
    fn mkdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;
    fn rmdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;
    fn readdir(&self, path: &str, tid: TaskId) -> Result<alloc::vec::Vec<DirEntry>, &'static str>;

    // Metadata & Permissions
    fn stat(&self, path: &str, tid: TaskId) -> Result<FileStats, &'static str>;
    fn chmod(&self, _path: &str, _mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }
    fn chown(&self, _path: &str, _uid: u32, _gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }

    // Links
    fn rename(&self, _old_path: &str, _new_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }
    fn link(&self, _old_path: &str, _new_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }
    fn symlink(&self, _target: &str, _link_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }
    fn readlink(&self, _path: &str, _tid: TaskId) -> Result<alloc::string::String, &'static str> {
        Err("operation not supported")
    }

    // Timestamps
    fn set_times(
        &self,
        _path: &str,
        _atime: u64,
        _mtime: u64,
        _tid: TaskId,
    ) -> Result<(), &'static str> {
        Err("operation not supported")
    }

    /// Sync all dirty data/metadata for the entire filesystem to stable storage.
    fn sync_fs(&self) -> Result<(), &'static str> {
        Ok(())
    }
}
