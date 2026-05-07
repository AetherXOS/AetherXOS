//! Open file handle implementation for tmpfs.

use alloc::sync::Arc;
use core::any::Any;
use spin::Mutex;

use crate::modules::vfs::{
    constants::*,
    types::{File, FileStats, PollEvents, SeekFrom},
};
use super::data::TmpFileData;

/// A handle to an open tmpfs file.
pub struct TmpFileHandle {
    pub data: Arc<Mutex<TmpFileData>>,
    pub pos: usize,
}

impl File for TmpFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let data = self.data.lock();
        if self.pos >= data.content.len() {
            return Ok(0);
        }
        let remaining = &data.content[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        drop(data);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let mut data = self.data.lock();
        let end = self.pos + buf.len();
        if end > data.content.len() {
            data.content.resize(end, 0);
        }
        data.content[self.pos..end].copy_from_slice(buf);
        drop(data);
        self.pos = end;
        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let data = self.data.lock();
        let size = data.content.len() as i64;
        drop(data);
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.pos as i64 + n,
            SeekFrom::End(n) => size + n,
        };
        if new_pos < 0 {
            return Err("EINVAL");
        }
        self.pos = new_pos as usize;
        Ok(self.pos as u64)
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        let mut data = self.data.lock();
        data.content.resize(size as usize, 0);
        Ok(())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        let data = self.data.lock();
        Ok(FileStats {
            size: data.content.len() as u64,
            mode: data.mode,
            uid: data.uid,
            gid: data.gid,
            atime: crate::modules::vfs::types::VfsTimespec { sec: data.atime, nsec: 0 },
            mtime: crate::modules::vfs::types::VfsTimespec { sec: data.mtime, nsec: 0 },
            ctime: crate::modules::vfs::types::VfsTimespec { sec: data.ctime, nsec: 0 },
            blksize: BLOCK_SIZE as u32,
            blocks: (data.content.len() as u64 + BLOCK_SHIFT as u64 - 1) / BLOCK_SHIFT as u64,
            ..FileStats::default()
        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::IN | PollEvents::OUT
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
