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

#[derive(Debug, Clone, Copy)]
pub struct MountRecord {
    pub id: usize,
    pub fs_kind: usize,
    pub path_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountError {
    InvalidPath,
    MountNotFound,
}

pub fn mount_ramfs(_path: &[u8]) -> Result<usize, MountError> {
    Err(MountError::InvalidPath)
}

pub fn mount_count() -> usize {
    0
}

#[inline(always)]
pub fn mount_ramfs_typed(path: &[u8]) -> Result<super::MountId, MountError> {
    mount_ramfs(path).map(super::MountId)
}

#[inline(always)]
pub fn unmount_typed(mount_id: super::MountId) -> Result<(), MountError> {
    unmount(mount_id.0)
}

#[inline(always)]
pub fn mount_path_by_id_typed(mount_id: super::MountId, out: &mut [u8]) -> Option<usize> {
    mount_path_by_id(mount_id.0, out)
}

#[inline(always)]
pub fn mount_id_by_path_typed(path: &[u8]) -> Option<super::MountId> {
    mount_id_by_path(path).map(super::MountId)
}

#[inline(always)]
pub fn load_initrd_entries_typed(
    mount_id: super::MountId,
    entries: &[(&str, &[u8])],
) -> Result<usize, &'static str> {
    load_initrd_entries(mount_id.0, entries)
}

pub fn unmount(_mount_id: usize) -> Result<(), MountError> {
    Err(MountError::MountNotFound)
}

pub fn unmount_by_path(_path: &[u8]) -> Result<(), MountError> {
    Err(MountError::MountNotFound)
}

pub fn list_mounts(_out: &mut [MountRecord]) -> usize {
    0
}

pub fn mount_path_by_id(_mount_id: usize, _out: &mut [u8]) -> Option<usize> {
    None
}

pub fn mount_id_by_path(_path: &[u8]) -> Option<usize> {
    None
}

pub fn relocate_mount(_mount_id: usize, _new_path: &[u8]) -> Result<(), MountError> {
    Err(MountError::MountNotFound)
}

pub fn set_mount_readonly(_mount_id: usize, _readonly: bool) -> Result<(), MountError> {
    Err(MountError::MountNotFound)
}

pub fn mount_readonly_by_path(_path: &[u8]) -> Option<bool> {
    None
}

pub fn mount_readonly_by_id(_mount_id: usize) -> Option<bool> {
    None
}

pub fn load_initrd_entries(
    _mount_id: usize,
    _entries: &[(&str, &[u8])],
) -> Result<usize, &'static str> {
    Err("vfs disabled")
}

pub fn stats() -> MountStats {
    MountStats {
        mount_attempts: 0,
        mount_success: 0,
        mount_failures: 0,
        unmount_attempts: 0,
        unmount_success: 0,
        unmount_failures: 0,
        unmount_by_path_attempts: 0,
        unmount_by_path_success: 0,
        unmount_by_path_failures: 0,
        path_validation_failures: 0,
        initrd_load_calls: 0,
        initrd_load_files: 0,
        initrd_load_bytes: 0,
        initrd_load_failures: 0,
        total_mounts: 0,
        last_mount_id: 0,
    }
}
