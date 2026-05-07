#[cfg(feature = "vfs")]
use alloc::vec::Vec;
#[cfg(feature = "vfs")]
use crate::interfaces::TaskId;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MountId(pub usize);

impl MountId {
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[cfg(feature = "vfs")]
#[derive(Debug, Clone, Copy)]
pub struct MountStats {
    pub mount_attempts: u64,
    pub mount_success: u64,
    pub mount_failures: u64,
    pub unmount_attempts: u64,
    pub unmount_success: u64,
    pub unmount_failures: u64,
    pub unmount_by_path_attempts: u64,
    pub unmount_by_path_success: u64,
    pub unmount_by_path_failures: u64,
    pub path_validation_failures: u64,
    pub initrd_load_calls: u64,
    pub initrd_load_files: u64,
    pub initrd_load_bytes: u64,
    pub initrd_load_failures: u64,
    pub total_mounts: usize,
    pub last_mount_id: usize,
}

#[cfg(feature = "vfs")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum MountFsKind {
    RamFs = 1,
    Ext4 = 2,
    Fat32 = 3,
    Overlay = 4,
}

#[cfg(feature = "vfs")]
#[derive(Debug, Clone, Copy)]
pub struct MountRecord {
    pub id: usize,
    pub fs_kind: usize,
    pub path_len: usize,
}

#[cfg(feature = "vfs")]
#[derive(Debug, Clone)]
pub(crate) struct MountEntry {
    pub(crate) id: usize,
    pub(crate) fs_kind: MountFsKind,
    pub(crate) path: Vec<u8>,
    pub(crate) path_len: usize,
    pub(crate) owner: TaskId,
    pub(crate) readonly: bool,
}

#[cfg(feature = "vfs")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountError {
    InvalidPath,
    AlreadyMounted,
    RegistryFull,
    MountNotFound,
}
