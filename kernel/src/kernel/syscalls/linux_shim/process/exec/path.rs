use super::super::super::*;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;

#[cfg(not(feature = "linux_compat"))]
pub fn resolve_execveat_path(dirfd: isize, path: &str, flags: usize) -> Result<alloc::string::String, usize> {
    const AT_EMPTY_PATH: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_EMPTY_PATH;

    if path.is_empty() {
        if (flags & AT_EMPTY_PATH) == 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
        }
        if dirfd < 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
        }

        #[cfg(feature = "posix_fs")]
        {
            return crate::modules::posix::fs::fd_path(dirfd as u32)
                .map_err(|err| linux_errno(err.code()));
        }

        #[cfg(not(feature = "posix_fs"))]
        {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOSYS));
        }
    }

    if path.starts_with('/') || dirfd == LINUX_AT_FDCWD {
        return Ok(path.into());
    }
    if dirfd < 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::fd_fs_context(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        let dir_path = crate::modules::posix::fs::fd_path(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        return crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, path)
            .map_err(|err| linux_errno(err.code()));
    }

    #[cfg(not(feature = "posix_fs"))]
    {
        Err(linux_errno(crate::modules::posix_consts::errno::ENOSYS))
    }
}
