use super::super::super::*;
use crate::kernel::syscalls::linux_shim::fs::support::{resolve_path_at_allow_empty, LINUX_AT_FDCWD, LINUX_AT_SYMLINK_NOFOLLOW};

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_chmod(path_ptr: usize, mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fchmod(fd: usize, mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fchmod(fd as u32, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, mode);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_chown(path_ptr: usize, uid: usize, gid: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chown(fs_id, &resolved, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, uid, gid);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fchown(fd: usize, uid: usize, gid: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fchown(fd as u32, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, uid, gid);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fchmodat(
    dirfd: usize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if (flags & !LINUX_AT_SYMLINK_NOFOLLOW) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let (fs_id, resolved) = match resolve_path_at_allow_empty(dirfd as isize, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, mode, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fchownat(
    dirfd: usize,
    path_ptr: usize,
    uid: usize,
    gid: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if (flags & !LINUX_AT_SYMLINK_NOFOLLOW) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let (fs_id, resolved) = match resolve_path_at_allow_empty(dirfd as isize, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chown(fs_id, &resolved, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, uid, gid, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}
