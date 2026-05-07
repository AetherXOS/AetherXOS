use super::super::super::super::util::read_user_path_like_string;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::fs::support::resolve_path_at;

pub(crate) fn sys_linux_mkdirat(dirfd: isize, path_ptr: usize, mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at(dirfd, path_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::mkdir(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_unlinkat(dirfd: isize, path_ptr: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        const AT_REMOVEDIR: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_REMOVEDIR;
        if (flags & !AT_REMOVEDIR) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let (fs_id, resolved) = match resolve_path_at(dirfd, path_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let result = if (flags & AT_REMOVEDIR) != 0 {
            crate::modules::posix::fs::rmdir(fs_id, &resolved)
        } else {
            crate::modules::posix::fs::unlink(fs_id, &resolved)
        };
        match result {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_linkat(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        const AT_SYMLINK_FOLLOW: usize = 0x400;
        if (flags & !AT_SYMLINK_FOLLOW) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }

        let (old_fs_id, old_resolved) = match resolve_path_at(olddirfd, oldpath_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let (new_fs_id, new_resolved) = match resolve_path_at(newdirfd, newpath_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if old_fs_id != new_fs_id {
            return linux_errno(crate::modules::posix_consts::errno::EXDEV);
        }

        match crate::modules::posix::fs::link(old_fs_id, &old_resolved, &new_resolved) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (olddirfd, oldpath_ptr, newdirfd, newpath_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_symlinkat(target_ptr: usize, newdirfd: isize, linkpath_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let target = match read_user_path_like_string(target_ptr) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let (fs_id, link_resolved) = match resolve_path_at(newdirfd, linkpath_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };

        match crate::modules::posix::fs::symlink(fs_id, &target, &link_resolved) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (target_ptr, newdirfd, linkpath_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}
