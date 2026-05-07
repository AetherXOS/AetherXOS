//! In-memory file node data for tmpfs.

use alloc::vec::Vec;

/// In-memory file node data.
pub struct TmpFileData {
    pub content: Vec<u8>,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
}

impl TmpFileData {
    pub fn new(mode: u32) -> Self {
        Self {
            content: Vec::new(),
            mode,
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
        }
    }
}
