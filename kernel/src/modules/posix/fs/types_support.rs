use super::*;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekWhence {
    Set = crate::modules::posix_consts::fs::SEEK_SET,
    Cur = crate::modules::posix_consts::fs::SEEK_CUR,
    End = crate::modules::posix_consts::fs::SEEK_END,
}

impl SeekWhence {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::fs::SEEK_SET => Some(Self::Set),
            crate::modules::posix_consts::fs::SEEK_CUR => Some(Self::Cur),
            crate::modules::posix_consts::fs::SEEK_END => Some(Self::End),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PosixStat {
    pub size: u64,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub ino: u64,
    pub atime: i64,
    pub mtime: i64,
    pub ctime: i64,
}

pub struct SharedFile {
    pub fs_id: u32,
    pub path: String,
    pub handle: Arc<Mutex<dyn crate::modules::vfs::File>>,
    pub offset: Mutex<u64>,
    pub flags: Mutex<u32>, // O_APPEND, etc.
}

#[derive(Clone)]
pub struct PosixFileDesc {
    pub file: Arc<SharedFile>,
    pub cloexec: bool,
}

#[derive(Debug, Clone)]
pub struct PosixMapDesc {
    pub(super) fs_id: u32,
    pub(super) path: String,
    pub(super) offset: usize,
    pub(super) len: usize,
    pub(super) writable: bool,
    pub(super) dirty: bool,
    pub(super) data: Arc<Mutex<alloc::vec::Vec<u8>>>,
    #[allow(dead_code)]
    pub(super) shared: bool,
}