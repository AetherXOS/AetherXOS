#[cfg(not(feature = "linux_compat"))]
use super::storage::{linux_fd_clear_descriptor_flags, linux_fd_set_descriptor_flags, LINUX_FD_CLOEXEC};
#[cfg(not(feature = "linux_compat"))]
use super::super::super::super::linux_errno;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pipe(pipefd_ptr: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_pipe")]
    {
        let nonblock = (flags & 0x800) != 0;
        let cloexec = (flags & 0x80000) != 0;
        match crate::modules::posix::pipe::pipe2(nonblock) {
            Ok((rfd, wfd)) => {
                let rfd_u32 = rfd as u32;
                let wfd_u32 = wfd as u32;
                if write_user_pod(pipefd_ptr, &rfd_u32).is_err()
                    || write_user_pod(pipefd_ptr + core::mem::size_of::<u32>(), &wfd_u32).is_err()
                {
                    return linux_errno(crate::modules::posix_consts::errno::EFAULT);
                }
                if cloexec {
                    linux_fd_set_descriptor_flags(rfd_u32, LINUX_FD_CLOEXEC);
                    linux_fd_set_descriptor_flags(wfd_u32, LINUX_FD_CLOEXEC);
                }
                0
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_pipe"))]
    {
        let _ = (pipefd_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::EMFILE)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup(oldfd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::dup(oldfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::libnet::posix_dup_errno(oldfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = oldfd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup2(oldfd: usize, newfd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::dup2(oldfd as u32, newfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::libnet::posix_dup2_errno(oldfd as u32, newfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = (oldfd, newfd);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup3(oldfd: usize, newfd: usize, flags: usize) -> usize {
    const O_CLOEXEC: usize = 0x80000;

    if oldfd == newfd {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !O_CLOEXEC) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let duped = sys_linux_dup2(oldfd, newfd);
    if duped == linux_errno(crate::modules::posix_consts::errno::EBADF)
        || duped == linux_errno(crate::modules::posix_consts::errno::EINVAL)
    {
        return duped;
    }

    if (flags & O_CLOEXEC) != 0 {
        linux_fd_set_descriptor_flags(newfd as u32, LINUX_FD_CLOEXEC);
    } else {
        linux_fd_clear_descriptor_flags(newfd as u32);
    }
    duped
}
