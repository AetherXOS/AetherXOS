#[cfg(feature = "posix_fs")]
use super::super::util::{read_user_c_string_allow_empty, read_user_path_like_string};
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use crate::kernel::syscalls::linux_errno;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) const LINUX_AT_SYMLINK_NOFOLLOW: usize = 0x100;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) const LINUX_AT_EACCESS: usize = 0x200;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) const LINUX_AT_NO_AUTOMOUNT: usize = 0x800;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) const LINUX_AT_EMPTY_PATH: usize = 0x1000;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) fn resolve_dirfd_context(
    dirfd: isize,
    path: &str,
) -> Result<(u32, alloc::string::String), usize> {
    if path.starts_with('/') || dirfd == LINUX_AT_FDCWD {
        let fs_id =
            crate::modules::posix::fs::default_fs_id().map_err(|err| linux_errno(err.code()))?;
        Ok((fs_id, alloc::string::String::from("/")))
    } else if dirfd >= 0 {
        let fs_id = crate::modules::posix::fs::fd_fs_context(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        let dir_path = crate::modules::posix::fs::fd_path(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        Ok((fs_id, dir_path))
    } else {
        Err(linux_errno(crate::modules::posix_consts::errno::EBADF))
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) fn resolve_path_at(
    dirfd: isize,
    path_ptr: usize,
) -> Result<(u32, alloc::string::String), usize> {
    let path = read_user_path_like_string(path_ptr)?;
    let (fs_id, dir_path) = resolve_dirfd_context(dirfd, &path)?;
    let resolved = if path.starts_with('/') {
        crate::modules::posix::fs::resolve_at_path(fs_id, "/", &path)
    } else {
        crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path)
    }
    .map_err(|err| linux_errno(err.code()))?;
    Ok((fs_id, resolved))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) fn resolve_path_at_with_flags(
    dirfd: isize,
    path_ptr: usize,
    flags: usize,
) -> Result<(u32, alloc::string::String), usize> {
    let allow_empty = (flags & LINUX_AT_EMPTY_PATH) != 0;
    let path = if allow_empty {
        read_user_c_string_allow_empty(
            path_ptr,
            crate::config::KernelConfig::syscall_max_path_len(),
        )?
    } else {
        read_user_path_like_string(path_ptr)?
    };

    if path.is_empty() {
        if !allow_empty {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
        }
        if dirfd < 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
        }
        let fs_id = crate::modules::posix::fs::fd_fs_context(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        let resolved = crate::modules::posix::fs::fd_path(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        return Ok((fs_id, resolved));
    }

    let (fs_id, dir_path) = resolve_dirfd_context(dirfd, &path)?;
    let resolved = if path.starts_with('/') {
        crate::modules::posix::fs::resolve_at_path(fs_id, "/", &path)
    } else {
        crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path)
    }
    .map_err(|err| linux_errno(err.code()))?;
    Ok((fs_id, resolved))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) fn resolve_path_at_allow_empty(
    dirfd: isize,
    pathname_ptr: usize,
    allow_empty: bool,
) -> Result<(u32, alloc::string::String), usize> {
    let flags = if allow_empty { LINUX_AT_EMPTY_PATH } else { 0 };
    resolve_path_at_with_flags(dirfd, pathname_ptr, flags)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(super) fn validate_newfstatat_flags(flags: usize) -> Result<(), usize> {
    let allowed = LINUX_AT_EMPTY_PATH | LINUX_AT_SYMLINK_NOFOLLOW | LINUX_AT_NO_AUTOMOUNT;
    if (flags & !allowed) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_fs"))]
mod tests {
    use super::*;

    #[test_case]
    fn validate_newfstatat_flags_rejects_unknown_bits() {
        assert_eq!(
            validate_newfstatat_flags(0x4000),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn resolve_path_at_with_flags_rejects_empty_path_without_allow_empty() {
        let path = b"\0";
        assert_eq!(
            resolve_path_at_with_flags(LINUX_AT_FDCWD, path.as_ptr() as usize, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::ENOENT))
        );
    }
}
