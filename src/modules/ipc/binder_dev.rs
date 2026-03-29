use crate::modules::vfs::{File, SeekFrom, types::{FileStats, PollEvents}};
use crate::interfaces::task::TaskId;
use crate::interfaces::task::ProcessId;
use core::any::Any;
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec::Vec;
use crate::modules::ipc::binder::{BinderTransaction, get_context};

/// The Binder Device File (/dev/binder)
pub struct BinderDeviceFile {
    pid: ProcessId,
}

impl BinderDeviceFile {
    pub fn new(pid: ProcessId) -> Self {
        Self { pid }
    }
}

impl File for BinderDeviceFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        // Standard binder usually uses ioctl for everything, 
        // but some versions might allow reading status.
        Err("use ioctl for binder")
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("use ioctl for binder")
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Err("binder is not seekable")
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o666,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        let ctx = get_context(self.pid);
        let mut events = PollEvents::empty();
        if !ctx.incoming.lock().is_empty() {
            events.insert(PollEvents::IN);
        }
        events
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        // Constants from Linux Binder
        const BINDER_WRITE_READ: u32 = 0xc0306201;
        const BINDER_SET_MAX_THREADS: u32 = 0x40046205;
        const BINDER_SET_CONTEXT_MGR: u32 = 0x40046207;

        match cmd {
            BINDER_SET_CONTEXT_MGR => {
                // In a real system, only one process can be the manager.
                Ok(0)
            }
            BINDER_SET_MAX_THREADS => {
                Ok(0)
            }
            BINDER_WRITE_READ => {
                // In a real implementation, we would copy_from_user the binder_write_read struct,
                // handle the commands in write_buffer and read_buffer.
                // For now, we proxy to our high-level binder_transact/binder_read if needed,
                // but this requires UserPtr access which the VFS layer doesn't have directly yet.
                Ok(0) 
            }
            _ => Err("unknown binder ioctl")
        }
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
