use crate::interfaces::TaskId;
use crate::modules::vfs::devfs::{DevFs, DevFsEvent, DevFsEventSnapshot, DeviceMetadata};
use crate::modules::vfs::FileSystem;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

use super::{time::PosixTimespec, PosixErrno};

#[path = "fs/devfs_support.rs"]
mod devfs_support;
#[path = "fs/fs_support.rs"]
mod fs_support;
#[path = "fs/fd_support.rs"]
mod fd_support;
#[path = "fs/io_support.rs"]
mod io_support;
#[path = "fs/inotify_support.rs"]
mod inotify_support;
#[path = "fs/mmap_support.rs"]
mod mmap_support;
#[path = "fs/path_support.rs"]
mod path_support;
#[path = "fs/state_support.rs"]
mod state_support;
#[path = "fs/metadata_support.rs"]
mod metadata_support;
#[path = "fs/allocation_support.rs"]
mod allocation_support;
#[path = "fs/devfs_api_support.rs"]
mod devfs_api_support;
#[path = "fs/fd_admin_support.rs"]
mod fd_admin_support;
#[path = "fs/file_ops_support.rs"]
mod file_ops_support;
#[path = "fs/lifecycle_support.rs"]
mod lifecycle_support;
#[path = "fs/types_support.rs"]
mod types_support;
#[path = "fs/file_types_support.rs"]
mod file_types_support;
pub use state_support::{
    CWD_INDEX, DEVFS_CONTEXTS, DIR_TABLE, FILE_INDEX, FILE_TABLE, FS_CONTEXTS, MMAP_TABLE,
    NEXT_DIRFD, NEXT_FD, NEXT_FS_ID, NEXT_MAP_ID, POSIX_DESCRIPTOR_CLOEXEC,
    POSIX_SUPPORTED_STATUS_FLAGS, SHM_FS_ID, UMASK_BITS,
};
pub use types_support::{PosixFileDesc, PosixMapDesc, PosixStat, SeekWhence, SharedFile};

use devfs_support::{devfs_context, register_builtin_devfs_nodes, sync_devfs_runtime_nodes};
use fs_support::{apply_devfs_policy_mode, map_fs_error, normalize_path};
pub use fd_support::{
    fcntl_get_descriptor_flags, fcntl_get_status_flags, fcntl_set_descriptor_flags,
    fcntl_set_status_flags, get_file_description, ioctl, register_file_description,
};
pub use inotify_support::{inotify_add_watch, inotify_init, inotify_rm_watch};
pub use mmap_support::{mmap, mmap_read, mmap_write, msync, munmap};
pub use path_support::{
    chdir, faccessat, fstatat, getcwd, linkat, mkdirat, openat, readlinkat, realpath, renameat,
    resolve_at_path, symlinkat, unlinkat,
};

pub use io_support::{
    close, creat, dup, dup2, fdatasync, fsync, lseek, open, pread, pwrite, read, readv,
    register_handle, register_posix_handle, write, writev,
};

pub use metadata_support::{
    access, chmod, chown, closedir, fchmod, fchown, fstat, futimens, futimes, link, lstat,
    mkdir, opendir, poll, readlink, readdir, rename, rmdir, scandir, stat, statfs, symlink,
    truncate, ftruncate, utimensat, utimes, PosixFsStats,
};
pub use allocation_support::{
    fallocate, fd_fs_context, fd_path, posix_fallocate, posix_fallocate_range, syncfs,
};
pub use devfs_api_support::{devfs_event_snapshot, devfs_events_since};
pub use fd_admin_support::{dup_at_least, umask};
pub use file_ops_support::{copy_file_range, unlink};
pub use lifecycle_support::{default_fs_id, mount_devfs, mount_ramfs, unmount};

#[cfg(test)]
mod fs_tests;
