pub mod operations;
pub mod registry;
pub mod stats;
pub mod types;

#[cfg(feature = "vfs")]
mod support;

#[cfg(feature = "vfs")]
mod ramfs;

#[cfg(not(feature = "vfs"))]
mod stubs;

pub use operations::*;
pub(crate) use registry::*;
pub use stats::*;
pub use types::*;

pub const ERR_MOUNT_NOT_FOUND: &'static str = "mount not found";

#[cfg(feature = "vfs")]
pub(crate) use self::support::{valid_initrd_path, ROOT_TASK_ID};

#[cfg(feature = "vfs")]
pub use ramfs::{
    load_initrd_entries, ramfs_chmod, ramfs_chown, ramfs_create_file, ramfs_link, ramfs_metadata,
    ramfs_mkdir, ramfs_open_file, ramfs_readdir, ramfs_readlink, ramfs_remove_file,
    ramfs_rename, ramfs_rmdir, ramfs_set_times, ramfs_symlink, ramfs_used_pages,
};

#[cfg(not(feature = "vfs"))]
pub use stubs::*;

#[cfg(feature = "vfs")]
#[inline(always)]
pub fn mount_ramfs_typed(path: &[u8]) -> Result<MountId, MountError> {
    mount_ramfs(path).map(MountId)
}

#[cfg(feature = "vfs")]
#[inline(always)]
pub fn unmount_typed(mount_id: MountId) -> Result<(), MountError> {
    unmount(mount_id.0)
}

#[cfg(feature = "vfs")]
#[inline(always)]
pub fn mount_path_by_id_typed(mount_id: MountId, out: &mut [u8]) -> Option<usize> {
    mount_path_by_id(mount_id.0, out)
}

#[cfg(feature = "vfs")]
#[inline(always)]
pub fn mount_id_by_path_typed(path: &[u8]) -> Option<MountId> {
    mount_id_by_path(path).map(MountId)
}

#[cfg(feature = "vfs")]
#[inline(always)]
pub fn load_initrd_entries_typed(
    mount_id: MountId,
    entries: &[(&str, &[u8])],
) -> Result<usize, &'static str> {
    load_initrd_entries(mount_id.0, entries)
}

#[cfg(all(test, feature = "vfs"))]
mod tests;
