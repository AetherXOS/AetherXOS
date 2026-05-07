use super::super::super::super::util::{read_user_path_like_string, read_user_pod};
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::{LINUX_O_CREAT, LINUX_O_EXCL, LINUX_O_TRUNC, LINUX_O_APPEND};
use crate::kernel::syscalls::linux_shim::fs::support::{resolve_dirfd_context, resolve_path_at};
#[cfg(feature = "posix_fs")]
use alloc::string::String;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxOpenHowCompat {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

#[cfg(feature = "posix_fs")]
pub fn resolved_open_path(dir_path: &str, path: &str) -> String {
    if path.starts_with('/') {
        String::from(path)
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

#[cfg(feature = "posix_fs")]
fn contains_parent_ref(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

#[cfg(feature = "posix_fs")]
#[inline]
pub fn path_escapes_dirfd(path: &str) -> bool {
    path.starts_with('/') || contains_parent_ref(path)
}

#[cfg(feature = "posix_fs")]
fn is_proc_magiclink_path(path: &str) -> bool {
    crate::modules::posix::fs::path_has_magiclink_component(path).unwrap_or(false)
}

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

        if (flags & crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CLOEXEC) != 0 {
            crate::kernel::syscalls::linux_shim::fd_process_identity::storage::linux_fd_set_descriptor_flags(
                fd as u32,
                crate::kernel::syscalls::linux_shim::fd_process_identity::storage::LINUX_FD_CLOEXEC,
            );
        }

        fd as usize
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, pathname_ptr, flags, _mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

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
