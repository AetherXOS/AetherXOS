use crate::modules::vfs::types::{File, FileStats, PollEvents, SeekFrom};
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

/// A simple read-only in-memory file backed by a dynamically generated buffer.
/// Shared by procfs, sysfs, and other virtual filesystems.
pub struct ReadOnlyFile {
    pub data: Vec<u8>,
    pub pos: usize,
}

impl ReadOnlyFile {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    pub fn from_string(s: String) -> Self {
        Self {
            data: s.into_bytes(),
            pos: 0,
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        Self {
            data: s.as_bytes().to_vec(),
            pos: 0,
        }
    }
}

impl File for ReadOnlyFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let remaining = &self.data[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EROFS")
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        match pos {
            SeekFrom::Start(n) => {
                self.pos = n as usize;
            }
            SeekFrom::Current(n) => {
                self.pos = (self.pos as i64 + n) as usize;
            }
            SeekFrom::End(n) => {
                self.pos = (self.data.len() as i64 + n) as usize;
            }
        }
        Ok(self.pos as u64)
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: self.data.len() as u64,
            mode: 0o100444, // regular file, r--r--r--
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..FileStats::default()
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
