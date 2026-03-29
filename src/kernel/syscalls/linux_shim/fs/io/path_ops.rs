#[cfg(feature = "posix_fs")]
use super::super::super::util::read_user_path_like_string;
use super::super::super::*;
#[cfg(feature = "posix_fs")]
use super::super::support::{
    resolve_dirfd_context, resolve_path_at, resolve_path_at_with_flags, LINUX_AT_EACCESS,
    LINUX_AT_EMPTY_PATH, LINUX_AT_SYMLINK_NOFOLLOW,
};

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxOpenHowCompat {
    flags: u64,
    mode: u64,
    resolve: u64,
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

    let how = match with_user_read_bytes(how_ptr, core::mem::size_of::<LinuxOpenHowCompat>(), |src| {
        let mut out = LinuxOpenHowCompat::default();
        let dst = unsafe {
            core::slice::from_raw_parts_mut(
                (&mut out as *mut LinuxOpenHowCompat).cast::<u8>(),
                core::mem::size_of::<LinuxOpenHowCompat>(),
            )
        };
        dst.copy_from_slice(src);
        out
    }) {
        Ok(v) => v,
        Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
    };

    let allowed_resolve = crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_ALLOWED_MASK as u64;
    if (how.resolve & !allowed_resolve) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Reuse openat semantics; resolve constraints are validated but not yet fully enforced.
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
    // Minimal compatibility: accept classic rename behavior when flags==0.
    if flags == 0 {
        return sys_linux_renameat(olddirfd, oldpath_ptr, newdirfd, newpath_ptr);
    }
    // RENAME_NOREPLACE / RENAME_EXCHANGE / RENAME_WHITEOUT not implemented here yet.
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
mod tests {
    use super::*;
    #[cfg(feature = "posix_fs")]
    use crate::kernel::syscalls::linux_shim::fs::support::LINUX_AT_EMPTY_PATH;

    #[test_case]
    fn openat_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_openat(LINUX_AT_FDCWD, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn faccessat_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_faccessat(LINUX_AT_FDCWD, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn faccessat_rejects_unknown_flags() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_faccessat(LINUX_AT_FDCWD, path.as_ptr() as usize, 0, 0x8000),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn faccessat_empty_path_without_fd_returns_ebadf() {
        #[cfg(feature = "posix_fs")]
        {
            let empty = b"\0";
            assert_eq!(
                sys_linux_faccessat(-2, empty.as_ptr() as usize, 0, LINUX_AT_EMPTY_PATH),
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            );
        }
    }

    #[test_case]
    fn faccessat_empty_path_uses_dirfd_context() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_faccessat_empty")
                .expect("mount");
            let fd = crate::modules::posix::fs::open(fs_id, "/visible", true).expect("open");
            let empty = b"\0";
            assert_eq!(
                sys_linux_faccessat(fd as isize, empty.as_ptr() as usize, 0, LINUX_AT_EMPTY_PATH),
                0
            );
            let _ = crate::modules::posix::fs::close(fd);
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn mkdirat_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_mkdirat(LINUX_AT_FDCWD, 0, 0o755),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn openat_invalid_dirfd_returns_ebadf_for_relative_paths() {
        let path = b"relative\0";
        assert_eq!(
            sys_linux_openat(-2, path.as_ptr() as usize, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }

    #[test_case]
    fn unlinkat_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_unlinkat(LINUX_AT_FDCWD, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn linkat_invalid_oldpath_pointer_returns_efault() {
        let newp = b"/tmp_new\0";
        assert_eq!(
            sys_linux_linkat(
                LINUX_AT_FDCWD,
                0,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                0,
            ),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn symlinkat_invalid_target_pointer_returns_efault() {
        let link = b"/tmp_link\0";
        assert_eq!(
            sys_linux_symlinkat(0, LINUX_AT_FDCWD, link.as_ptr() as usize),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn renameat_invalid_path_pointer_returns_efault() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_renameat(LINUX_AT_FDCWD, 0, LINUX_AT_FDCWD, path.as_ptr() as usize,),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn readlinkat_invalid_buffer_pointer_returns_efault() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id =
                crate::modules::posix::fs::mount_ramfs("/linux_shim_readlink").expect("mount");
            let _ = crate::modules::posix::fs::symlink(fs_id, "/target", "/ln");
            let path = b"/ln\0";
            assert_eq!(
                sys_linux_readlinkat(LINUX_AT_FDCWD, path.as_ptr() as usize, 0x1, 8,),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn readlinkat_truncates_to_caller_buffer_length() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_readlink_truncate")
                .expect("mount");
            let _ = crate::modules::posix::fs::symlink(fs_id, "/long-target", "/ln");
            let path = b"/ln\0";
            let mut buf = [0u8; 4];
            assert_eq!(
                sys_linux_readlinkat(
                    LINUX_AT_FDCWD,
                    path.as_ptr() as usize,
                    buf.as_mut_ptr() as usize,
                    buf.len(),
                ),
                buf.len()
            );
            assert_eq!(&buf, b"/lon");
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn renameat2_rejects_nonzero_flags_in_minimal_mode() {
        let oldp = b"/old\0";
        let newp = b"/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                1,
            ),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
