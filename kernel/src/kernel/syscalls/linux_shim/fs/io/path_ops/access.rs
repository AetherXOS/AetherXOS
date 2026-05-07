use super::super::super::super::util::read_user_path_like_string;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::fs::support::{resolve_path_at_with_flags, LINUX_AT_EACCESS, LINUX_AT_EMPTY_PATH, LINUX_AT_SYMLINK_NOFOLLOW};

pub(crate) fn sys_linux_access(path_ptr: usize, _mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let path = match read_user_path_like_string(path_ptr) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(id) => id,
            Err(err) => return linux_errno(err.code()),
        };
        match crate::modules::posix::fs::access(fs_id, &path) {
            Ok(true) => 0,
            Ok(false) => linux_errno(crate::modules::posix_consts::errno::ENOENT),
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, _mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_faccessat(
    dirfd: isize,
    path_ptr: usize,
    _mode: usize,
    _flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let allowed_flags = LINUX_AT_EACCESS | LINUX_AT_SYMLINK_NOFOLLOW | LINUX_AT_EMPTY_PATH;
        if (_flags & !allowed_flags) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }

        let (fs_id, resolved) = match resolve_path_at_with_flags(dirfd, path_ptr, _flags) {
            Ok(v) => v,
            Err(err) => return err,
        };

        match crate::modules::posix::fs::access(fs_id, &resolved) {
            Ok(true) => 0,
            Ok(false) => linux_errno(crate::modules::posix_consts::errno::ENOENT),
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, _mode, _flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_faccessat2(
    dirfd: isize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    sys_linux_faccessat(dirfd, path_ptr, mode, flags)
}
