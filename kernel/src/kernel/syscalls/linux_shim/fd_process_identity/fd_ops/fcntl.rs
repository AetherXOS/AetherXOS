#[cfg(not(feature = "linux_compat"))]
use super::storage::{linux_fd_clear_descriptor_flags, linux_fd_get_descriptor_flags, linux_fd_set_descriptor_flags, LINUX_FD_CLOEXEC};
#[cfg(not(feature = "linux_compat"))]
use super::super::super::super::linux_errno;
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_shim::util::write_user_pod;
#[cfg(not(feature = "posix_process"))]
use super::super::super::task_time::sys_linux_getpid;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_fcntl(fd: usize, cmd: usize, arg: usize) -> usize {
    const F_DUPFD: usize = 0;
    const F_GETFD: usize = 1;
    const F_SETFD: usize = 2;
    const F_GETFL: usize = 3;
    const F_SETFL: usize = 4;
    const F_GETLK: usize = 5;
    const F_SETLK: usize = 6;
    const F_SETLKW: usize = 7;
    const F_SETOWN: usize = 8;
    const F_GETOWN: usize = 9;
    const F_OFD_GETLK: usize = 36;
    const F_OFD_SETLK: usize = 37;
    const F_OFD_SETLKW: usize = 38;
    const F_DUPFD_CLOEXEC: usize = 1030;
    const F_SETPIPE_SZ: usize = 1031;
    const F_GETPIPE_SZ: usize = 1032;
    const F_UNLCK: i16 = 2;
    const PIPE_BUF_SIZE: usize = 65536;

    match cmd {
        F_DUPFD => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::dup_at_least(fd as u32, arg as u32) {
                    Ok(newfd) => {
                        linux_fd_clear_descriptor_flags(newfd);
                        newfd as usize
                    }
                    Err(err) => linux_errno(err.code()),
                }
            }
            #[cfg(not(feature = "posix_fs"))]
            {
                let _ = arg;
                sys_linux_dup(fd)
            }
        }
        F_DUPFD_CLOEXEC => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::dup_at_least(fd as u32, arg as u32) {
                    Ok(newfd) => {
                        linux_fd_set_descriptor_flags(newfd, LINUX_FD_CLOEXEC);
                        newfd as usize
                    }
                    Err(err) => linux_errno(err.code()),
                }
            }
            #[cfg(not(feature = "posix_fs"))]
            {
                let _ = arg;
                sys_linux_dup(fd)
            }
        }
        F_GETFD => linux_fd_get_descriptor_flags(fd as u32) & LINUX_FD_CLOEXEC,
        F_SETFD => {
            linux_fd_set_descriptor_flags(fd as u32, arg);
            0
        }
        F_GETFL => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::fcntl_get_status_flags(fd as u32) {
                    Ok(flags) => return flags as usize,
                    Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(feature = "posix_net")]
            {
                match crate::modules::libnet::posix_fcntl_getfl_errno(fd as u32) {
                    Ok(flags) => return flags.bits() as usize,
                    Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
            {
                0o02
            }
            #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
            {
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            }
        }
        F_SETFL => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::fcntl_set_status_flags(fd as u32, arg as u32) {
                    Ok(()) => return 0,
                    Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(feature = "posix_net")]
            {
                let flags = crate::modules::libnet::PosixFdFlags::from_bits_truncate(arg as u32);
                match crate::modules::libnet::posix_fcntl_setfl_errno(fd as u32, flags) {
                    Ok(()) => return 0,
                    Err(crate::modules::libnet::PosixFdFlags::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
            {
                let _ = arg;
                0
            }
            #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
            {
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            }
        }
        F_GETLK | F_OFD_GETLK => {
            if arg == 0 {
                return linux_errno(crate::modules::posix_consts::errno::EFAULT);
            }
            write_user_pod(arg, &F_UNLCK)
                .map(|_| 0usize)
                .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
        }
        F_SETLK | F_SETLKW | F_OFD_SETLK | F_OFD_SETLKW => 0,
        F_GETOWN => {
            #[cfg(feature = "posix_process")]
            {
                crate::modules::posix::process::getpid() as usize
            }
            #[cfg(not(feature = "posix_process"))]
            {
                sys_linux_getpid()
            }
        }
        F_SETOWN => {
            let _ = arg;
            0
        }
        F_GETPIPE_SZ => PIPE_BUF_SIZE,
        F_SETPIPE_SZ => arg
            .max(4096)
            .min(linux_shim_pipe_set_max_size())
            .next_power_of_two(),
        _ => {
            let _ = (fd, arg);
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        }
    }
}

#[cfg(not(feature = "linux_compat"))]
fn linux_shim_pipe_set_max_size() -> usize {
    const MIN_CAP: usize = 64 * 1024;
    const MAX_CAP: usize = 16 * 1024 * 1024;

    crate::config::KernelConfig::launch_max_boot_image_bytes().clamp(MIN_CAP, MAX_CAP)
}
