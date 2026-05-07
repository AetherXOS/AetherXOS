use core::any::Any;
use crate::modules::vfs::types::{File, FileStats, PollEvents, SeekFrom};
use super::prng::{fill_random_bytes, PRNG_STATE};
use core::sync::atomic::Ordering;

/// `/dev/null` — reads return EOF, writes succeed silently.
pub struct DevNull;

impl File for DevNull {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(0)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Ok(0)
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..crate::modules::vfs::types::FileStats::default()
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::OUT
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `/dev/zero` — reads return zeroes, writes succeed silently.
pub struct DevZero;

impl File for DevZero {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Ok(0)
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..crate::modules::vfs::types::FileStats::default()
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

/// `/dev/full` — reads return zeroes, writes fail with ENOSPC.
pub struct DevFull;

impl File for DevFull {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("ENOSPC")
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..crate::modules::vfs::types::FileStats::default()
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::IN
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `/dev/random` and `/dev/urandom` — reads return pseudo-random bytes.
pub struct DevRandom;

impl File for DevRandom {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        fill_random_bytes(buf);
        Ok(buf.len())
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if !buf.is_empty() {
            let mut mix: u64 = 0;
            for (i, &b) in buf.iter().enumerate() {
                mix ^= (b as u64) << ((i % 8) * 8);
            }
            let old = PRNG_STATE.load(Ordering::Relaxed);
            PRNG_STATE.store(old ^ mix, Ordering::Relaxed);
        }
        Ok(buf.len())
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020444,
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..crate::modules::vfs::types::FileStats::default()
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
