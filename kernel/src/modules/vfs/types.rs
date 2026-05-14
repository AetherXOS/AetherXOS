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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VfsTimespec {
    pub sec: u64,
    pub nsec: u32,
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ResolveFlags: u32 {
        const NO_XDEV = 0x01;
        const NO_MAGICLINKS = 0x02;
        const NO_SYMLINKS = 0x04;
        const BENEATH = 0x08;
        const IN_ROOT = 0x10;
        const CACHED = 0x20;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPolicy {
    Buffered,
    Unbuffered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileStats {
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub atime: VfsTimespec,
    pub mtime: VfsTimespec,
    pub ctime: VfsTimespec,
    pub btime: VfsTimespec, // Birth time
    pub blksize: u32,
    pub blocks: u64,
    pub ino: u64,
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

/// The core File trait, now integrated with the KObject unified model.
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
    fn fsync(&mut self) -> Result<(), &'static str> {
        self.flush()
    }
    fn fdatasync(&mut self) -> Result<(), &'static str> {
        self.fsync()
    }
    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Err("truncate not supported")
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Err("stat not supported")
    }

    fn lock(&self, lock_type: LockType) -> Result<(), &'static str> {
        if lock_type == LockType::Unlock {
            Ok(())
        } else {
            Err("locking not supported")
        }
    }

    fn wait_queue(&self) -> Option<Arc<crate::kernel::sync::WaitQueue>> {
        None
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::IN | PollEvents::OUT
    }

    fn ioctl(&mut self, _cmd: u32, _arg: u64) -> Result<isize, &'static str> {
        Err("ioctl not supported")
    }

    fn mmap_physical(
        &self,
        _offset: u64,
        _len: usize,
    ) -> Result<alloc::vec::Vec<u64>, &'static str> {
        Err("physical mmap not supported")
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

    fn try_clone(&self) -> Result<Box<dyn File>, &'static str> {
        Err("clone not supported for this file type")
    }
}

pub trait FileSystem: Send + Sync {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str>;
    fn create(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str>;
    fn remove(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;

    fn mkdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;
    fn rmdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str>;
    fn readdir(&self, path: &str, tid: TaskId) -> Result<alloc::vec::Vec<DirEntry>, &'static str>;

    fn stat(&self, path: &str, tid: TaskId) -> Result<FileStats, &'static str>;
    fn chmod(&self, _path: &str, _mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }
    fn chown(&self, _path: &str, _uid: u32, _gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        Err("operation not supported")
    }

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

    fn set_times(
        &self,
        _path: &str,
        _atime: u64,
        _mtime: u64,
        _tid: TaskId,
    ) -> Result<(), &'static str> {
        Err("operation not supported")
    }

    fn sync_fs(&self) -> Result<(), &'static str> {
        Ok(())
    }

    fn statfs(&self, _path: &str, _tid: TaskId) -> Result<FsStats, &'static str> {
        Err("operation not supported")
    }

    fn lookup_dentry(&self, _path: &str) -> Option<Arc<crate::modules::vfs::cache::Dentry>> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FsStats {
    pub f_type: u64,
    pub f_bsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: u64,
    pub f_namelen: u64,
}
