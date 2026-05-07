use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use crate::modules::vfs::types::FileStats;

pub struct RamInode {
    pub stats: FileStats,
    pub data: Vec<u8>,
}

impl RamInode {
    pub fn new(mode: u16) -> Self {
        Self {
            stats: FileStats {
                size: 0,
                mode,
                uid: 0,
                gid: 0,
                atime: Default::default(),
                mtime: Default::default(),
                ctime: Default::default(),
                blksize: 4096,
                blocks: 0,
            },
            data: Vec::new(),
        }
    }

    pub fn read(&self, offset: u64, buf: &mut [u8]) -> usize {
        let size = self.data.len() as u64;
        if offset >= size { return 0; }
        let end = (offset + buf.len() as u64).min(size);
        let len = (end - offset) as usize;
        buf[..len].copy_from_slice(&self.data[offset as usize..end as usize]);
        len
    }

    pub fn write(&mut self, offset: u64, buf: &[u8]) -> usize {
        let end = offset + buf.len() as u64;
        if end > self.data.len() as u64 {
            self.data.resize(end as usize, 0);
        }
        self.data[offset as usize..end as usize].copy_from_slice(buf);
        self.stats.size = self.data.len() as u64;
        buf.len()
    }
}
