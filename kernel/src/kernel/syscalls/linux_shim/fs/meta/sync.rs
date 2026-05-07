use super::super::super::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fsync(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fsync(fd as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fdatasync(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fdatasync(fd as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_sync() -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if let Ok(fs_id) = crate::modules::posix::fs::default_fs_id() {
            let _ = crate::modules::posix::fs::syncfs(fs_id);
        }
        0
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_syncfs(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd as u32) {
            Ok(id) => id,
            Err(err) => return linux_errno(err.code()),
        };
        match crate::modules::posix::fs::syncfs(fs_id) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}
