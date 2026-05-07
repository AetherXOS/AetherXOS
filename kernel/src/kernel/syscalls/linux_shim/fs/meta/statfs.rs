use super::super::super::*;
use crate::kernel::syscalls::linux_shim::fs::support::{resolve_path_at_allow_empty, LINUX_AT_FDCWD};

#[repr(C)]
#[cfg(feature = "posix_fs")]
#[derive(Clone, Copy)]
pub(crate) struct LinuxStatfs {
    pub(crate) f_type: i64,
    pub(crate) f_bsize: i64,
    pub(crate) f_blocks: u64,
    pub(crate) f_bfree: u64,
    pub(crate) f_bavail: u64,
    pub(crate) f_files: u64,
    pub(crate) f_ffree: u64,
    pub(crate) f_fsid: [i32; 2],
    pub(crate) f_namelen: i64,
    pub(crate) f_frsize: i64,
    pub(crate) f_flags: i64,
    pub(crate) f_spare: [i64; 4],
}

#[cfg(feature = "posix_fs")]
pub(crate) fn fill_linux_statfs(fs_id: u32, stats: crate::modules::posix::fs::PosixFsStats) -> LinuxStatfs {
    LinuxStatfs {
        f_type: if fs_id == 1 {
            crate::kernel::syscalls::syscalls_consts::linux::RAMFS_MAGIC as i64
        } else {
            0xbeef_fadeu32 as i64
        },
        f_bsize: stats.f_bsize as i64,
        f_blocks: stats.f_blocks,
        f_bfree: stats.f_bfree,
        f_bavail: stats.f_bavail,
        f_files: stats.f_files,
        f_ffree: stats.f_ffree,
        f_fsid: [fs_id as i32, 0],
        f_namelen: stats.f_namelen as i64,
        f_frsize: stats.f_bsize as i64,
        f_flags: 0,
        f_spare: [0; 4],
    }
}

#[cfg(feature = "posix_fs")]
pub(crate) fn write_linux_statfs(buf_ptr: usize, stats: LinuxStatfs) -> usize {
    with_user_write_bytes(buf_ptr, core::mem::size_of::<LinuxStatfs>(), |dst| {
        let src_ptr = &stats as *const LinuxStatfs as *const u8;
        let src =
            unsafe { core::slice::from_raw_parts(src_ptr, core::mem::size_of::<LinuxStatfs>()) };
        dst.copy_from_slice(src);
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_statfs(path_ptr: usize, buf_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if let Err(err) = crate::modules::posix::fs::stat(fs_id, &resolved) {
            return linux_errno(err.code());
        }
        let stats = match crate::modules::posix::fs::statfs(fs_id) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_linux_statfs(buf_ptr, fill_linux_statfs(fs_id, stats))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, buf_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fstatfs(fd: usize, buf_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd as u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let stats = match crate::modules::posix::fs::statfs(fs_id) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_linux_statfs(buf_ptr, fill_linux_statfs(fs_id, stats))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, buf_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}
