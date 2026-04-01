#[cfg(feature = "posix_fs")]
use super::super::super::util::read_user_path_like_string;
#[cfg(feature = "posix_fs")]
use alloc::string::String;
use super::super::super::*;
#[cfg(feature = "posix_fs")]
use super::super::support::{
    resolve_dirfd_context, resolve_path_at, resolve_path_at_with_flags, LINUX_AT_EACCESS,
    LINUX_AT_EMPTY_PATH, LINUX_AT_SYMLINK_NOFOLLOW,
};
#[cfg(not(feature = "linux_compat"))]
use super::super::super::util::read_user_pod;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxOpenHowCompat {
    flags: u64,
    mode: u64,
    resolve: u64,
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn resolved_open_path(dir_path: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else if dir_path == "/" {
        alloc::format!("/{}", path.trim_start_matches('/'))
    } else {
        alloc::format!(
            "{}/{}",
            dir_path.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn contains_parent_ref(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
#[inline]
fn path_escapes_dirfd(path: &str) -> bool {
    path.starts_with('/') || contains_parent_ref(path)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn is_proc_magiclink_path(path: &str) -> bool {
    crate::modules::posix::fs::path_has_magiclink_component(path).unwrap_or(false)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_openat(
    dirfd: isize,
    pathname_ptr: usize,
    flags: usize,
    _mode: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let path = match read_user_path_like_string(pathname_ptr) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let (fs_id, dir_path) = match resolve_dirfd_context(dirfd, &path) {
            Ok(v) => v,
            Err(err) => return err,
        };

        if (flags & LINUX_O_CREAT) != 0 && (flags & LINUX_O_EXCL) != 0 {
            let existing = if path.starts_with('/') {
                crate::modules::posix::fs::access(fs_id, &path)
            } else {
                let resolved = resolved_open_path(&dir_path, &path);
                crate::modules::posix::fs::access(fs_id, &resolved)
            };
            match existing {
                Ok(true) => return linux_errno(crate::modules::posix_consts::errno::EEXIST),
                Ok(false) => {}
                Err(err) => return linux_errno(err.code()),
            }
        }

        let create = (flags & LINUX_O_CREAT) != 0;
        let fd_res = if path.starts_with('/') {
            crate::modules::posix::fs::open(fs_id, &path, create)
        } else {
            crate::modules::posix::fs::openat(fs_id, &dir_path, &path, create)
        };

        let fd = match fd_res {
            Ok(fd) => fd,
            Err(err) => return linux_errno(err.code()),
        };

        if (flags & LINUX_O_TRUNC) != 0 {
            if let Err(err) = crate::modules::posix::fs::ftruncate(fd, 0) {
                let _ = crate::modules::posix::fs::close(fd);
                return linux_errno(err.code());
            }
        }

        if (flags & LINUX_O_APPEND) != 0 {
            let current = crate::modules::posix::fs::fcntl_get_status_flags(fd).unwrap_or(0);
            let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                fd,
                current | crate::modules::posix_consts::fs::O_APPEND as u32,
            );
            let _ =
                crate::modules::posix::fs::lseek(fd, 0, crate::modules::posix::fs::SeekWhence::End);
        }

        fd as usize
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, pathname_ptr, flags, _mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_openat2(
    dirfd: isize,
    pathname_ptr: usize,
    how_ptr: usize,
    size: usize,
) -> usize {
    if how_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if size < core::mem::size_of::<LinuxOpenHowCompat>() || size > 4096 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let how = match read_user_pod::<LinuxOpenHowCompat>(how_ptr) {
        Ok(v) => v,
        Err(err) => return err,
    };

    let allowed_resolve = crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_ALLOWED_MASK as u64;
    if (how.resolve & !allowed_resolve) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        use crate::kernel::syscalls::syscalls_consts::linux::openat2;

        let path = match read_user_path_like_string(pathname_ptr) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let (fs_id, dir_path) = match resolve_dirfd_context(dirfd, &path) {
            Ok(v) => v,
            Err(err) => return err,
        };

        if (how.resolve & openat2::RESOLVE_CACHED as u64) != 0 {
            // Cache-only path walk requires a dentry cache contract we do not yet model.
            return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
        }

        if (how.resolve & (openat2::RESOLVE_BENEATH | openat2::RESOLVE_IN_ROOT) as u64) != 0
            && path_escapes_dirfd(&path)
        {
            return linux_errno(crate::modules::posix_consts::errno::EXDEV);
        }

        if (how.resolve & openat2::RESOLVE_NO_XDEV as u64) != 0 {
            if path_escapes_dirfd(&path) {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }

            let (resolved_fs_id, resolved_path) = match resolve_path_at(dirfd, pathname_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            if resolved_fs_id != fs_id {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }
            if resolved_path.starts_with("/proc/")
                || resolved_path.starts_with("/sys/")
                || resolved_path.starts_with("/dev/")
            {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }
        }

        if (how.resolve & openat2::RESOLVE_NO_MAGICLINKS as u64) != 0
            && is_proc_magiclink_path(&path)
        {
            return linux_errno(crate::modules::posix_consts::errno::ELOOP);
        }

        if (how.resolve & openat2::RESOLVE_NO_SYMLINKS as u64) != 0 {
            let resolved = resolved_open_path(&dir_path, &path);
            match crate::modules::posix::fs::path_contains_symlink_component(fs_id, &resolved, true)
            {
                Ok(true) => return linux_errno(crate::modules::posix_consts::errno::ELOOP),
                Ok(false) => {}
                Err(err) => return linux_errno(err.code()),
            }
        }
    }

    // Reuse openat for actual open/create once resolve policy checks are validated.
    sys_linux_openat(dirfd, pathname_ptr, how.flags as usize, how.mode as usize)
}

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_faccessat2(
    dirfd: isize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    sys_linux_faccessat(dirfd, path_ptr, mode, flags)
}

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
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

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_renameat(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
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
        match crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (olddirfd, oldpath_ptr, newdirfd, newpath_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_renameat2(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
    flags: usize,
) -> usize {
    const RENAME_NOREPLACE: usize = 1;
    const RENAME_EXCHANGE: usize = 2;
    const RENAME_WHITEOUT: usize = 4;
    let allowed_flags = RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT;

    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & RENAME_NOREPLACE) != 0 && (flags & RENAME_EXCHANGE) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Minimal compatibility: accept classic rename behavior when flags==0.
    if flags == 0 {
        return sys_linux_renameat(olddirfd, oldpath_ptr, newdirfd, newpath_ptr);
    }

    #[cfg(feature = "posix_fs")]
    {
        if flags == RENAME_NOREPLACE {
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

            match crate::modules::posix::fs::access(new_fs_id, &new_resolved) {
                Ok(true) => return linux_errno(crate::modules::posix_consts::errno::EEXIST),
                Ok(false) => {}
                Err(err) => return linux_errno(err.code()),
            }

            return match crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved)
            {
                Ok(()) => 0,
                Err(err) => linux_errno(err.code()),
            };
        }

        if flags == RENAME_EXCHANGE {
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

            let old_exists = match crate::modules::posix::fs::access(old_fs_id, &old_resolved) {
                Ok(exists) => exists,
                Err(err) => return linux_errno(err.code()),
            };
            let new_exists = match crate::modules::posix::fs::access(new_fs_id, &new_resolved) {
                Ok(exists) => exists,
                Err(err) => return linux_errno(err.code()),
            };
            if !old_exists || !new_exists {
                return linux_errno(crate::modules::posix_consts::errno::ENOENT);
            }

            // Best-effort exchange using a temporary sibling path.
            let mut tmp_path: Option<String> = None;
            for idx in 0..16u8 {
                let mut candidate = new_resolved.clone();
                candidate.push_str(".hc_swap_tmp_");
                let digit = if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + (idx - 10)) as char
                };
                candidate.push(digit);
                match crate::modules::posix::fs::access(new_fs_id, &candidate) {
                    Ok(false) => {
                        tmp_path = Some(candidate);
                        break;
                    }
                    Ok(true) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }

            let Some(tmp_resolved) = tmp_path else {
                return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
            };

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &tmp_resolved)
            {
                return linux_errno(err.code());
            }

            if let Err(err) = crate::modules::posix::fs::rename(new_fs_id, &new_resolved, &old_resolved)
            {
                let _ = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &old_resolved);
                return linux_errno(err.code());
            }

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &new_resolved)
            {
                let _ = crate::modules::posix::fs::rename(new_fs_id, &old_resolved, &new_resolved);
                let _ = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &old_resolved);
                return linux_errno(err.code());
            }

            return 0;
        }

        if flags == RENAME_WHITEOUT {
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

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved)
            {
                return linux_errno(err.code());
            }

            // Best-effort whiteout marker: recreate source path as hidden placeholder.
            // This keeps lower-layer style lookups blocked in simplified overlay paths.
            if let Err(err) = crate::modules::posix::fs::open(old_fs_id, &old_resolved, true) {
                return linux_errno(err.code());
            }
            let _ = crate::modules::posix::fs::chmod(old_fs_id, &old_resolved, 0o000);
            return 0;
        }
    }

    // Any combination not explicitly modeled above remains invalid for compatibility safety.
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_readlinkat(
    dirfd: isize,
    path_ptr: usize,
    buf_ptr: usize,
    buf_size: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at(dirfd, path_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let target = match crate::modules::posix::fs::readlink(fs_id, &resolved) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let out = target.as_bytes();
        let copy_len = core::cmp::min(buf_size, out.len());
        with_user_write_bytes(buf_ptr, copy_len, |dst| {
            dst.copy_from_slice(&out[..copy_len]);
            copy_len
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, buf_ptr, buf_size);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "path_ops/tests.rs"]
mod tests;
