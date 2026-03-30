use super::*;

#[path = "vfs/io_ops.rs"]
mod io_ops;
#[path = "vfs/management_ops.rs"]
mod management_ops;

pub(crate) use io_ops::{sys_vfs_close, sys_vfs_open, sys_vfs_read, sys_vfs_write};
pub(crate) use management_ops::{
    sys_vfs_get_mount_path, sys_vfs_get_stats, sys_vfs_list_mounts, sys_vfs_mount_diskfs,
    sys_vfs_mount_ramfs,
    sys_vfs_unmount, sys_vfs_unmount_path,
};
