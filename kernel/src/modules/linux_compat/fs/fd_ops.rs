use super::super::*;
use crate::modules::linux_compat::fs::io as fs_io;
use crate::modules::linux_compat::{linux, linux_errno, linux_fault, linux_inval, Fd, UserPtr};

const LINUX_CLOSE_RANGE_UNSHARE: usize = 1 << 1;
const LINUX_CLOSE_RANGE_CLOEXEC: usize = 1 << 2;

pub fn sys_linux_close_range(first: usize, last: usize, flags: usize) -> usize {
    if last < first {
        return linux_inval();
    }
    if (flags & !(LINUX_CLOSE_RANGE_UNSHARE | LINUX_CLOSE_RANGE_CLOEXEC)) != 0 {
        return linux_inval();
    }
    crate::require_posix_fs!((first, last, flags) => {
        // Per-task FD table isolation is not yet modeled; accept UNSHARE and
        // apply requested close/cloexec behavior on the current descriptor set.
        let _ = (flags & LINUX_CLOSE_RANGE_UNSHARE) != 0;
        let open_fds: alloc::vec::Vec<u32> = {
            let table = crate::modules::posix::fs::FILE_TABLE.lock();
            table
                .keys()
                .copied()
                .filter(|fd| (*fd as usize) >= first && (*fd as usize) <= last)
                .collect()
        };

        if (flags & LINUX_CLOSE_RANGE_CLOEXEC) != 0 {
            for fd in open_fds {
                fs_io::linux_fd_set_descriptor_flags(fd, fs_io::LINUX_FD_CLOEXEC);
            }
            return 0;
        }

        for fd in open_fds {
            let _ = sys_linux_close(Fd(fd as i32));
        }
        0
    })
}

pub fn sys_linux_pipe(pipefd_ptr: UserPtr<i32>) -> usize {
    sys_linux_pipe2(pipefd_ptr, 0)
}

pub fn sys_linux_pipe2(pipefd_ptr: UserPtr<i32>, flags: usize) -> usize {
    if pipefd_ptr.is_null() {
        return linux_fault();
    }

    let allowed_flags = linux::open_flags::O_NONBLOCK | linux::open_flags::O_CLOEXEC;
    if (flags & !allowed_flags) != 0 {
        return linux_inval();
    }

    crate::require_posix_pipe!((pipefd_ptr, flags) => {
        match crate::modules::posix::pipe::pipe2_flags(flags as i32) {
            Ok((rfd, wfd)) => {
                if (flags & linux::open_flags::O_CLOEXEC) != 0 {
                    fs_io::linux_fd_set_descriptor_flags(rfd, fs_io::LINUX_FD_CLOEXEC);
                    fs_io::linux_fd_set_descriptor_flags(wfd, fs_io::LINUX_FD_CLOEXEC);
                } else {
                    fs_io::linux_fd_clear_descriptor_flags(rfd);
                    fs_io::linux_fd_clear_descriptor_flags(wfd);
                }

                let fds = [rfd as i32, wfd as i32];
                let fds_bytes = unsafe {
                    core::slice::from_raw_parts(
                        fds.as_ptr() as *const u8,
                        core::mem::size_of_val(&fds),
                    )
                };
                match UserPtr::<u8>::new(pipefd_ptr.addr).write_bytes(fds_bytes) {
                    Ok(()) => 0,
                    Err(e) => {
                        let _ = sys_linux_close(Fd(rfd as i32));
                        let _ = sys_linux_close(Fd(wfd as i32));
                        e
                    }
                }
            }
            Err(e) => linux_errno(e as i32),
        }
    })
}

/// `close(2)` — Close a file descriptor.
pub fn sys_linux_close(fd: Fd) -> usize {
    if fd.as_usize() <= linux::STDERR_FILENO {
        return 0;
    }

    #[cfg(feature = "posix_pipe")]
    if fd.as_usize() >= linux::PIPE_BASE_FD {
        return match crate::modules::posix::pipe::close(fd.as_u32()) {
            Ok(()) => {
                fs_io::linux_fd_clear_descriptor_flags(fd.as_u32());
                0
            }
            Err(e) => linux_errno(e as i32),
        };
    }

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::close(fd.as_u32()) {
            Ok(()) => {
                fs_io::linux_fd_clear_descriptor_flags(fd.as_u32());
                0
            }
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

pub fn sys_linux_dup(oldfd: Fd) -> usize {
    crate::require_posix_fs!((oldfd) => {
        match crate::modules::posix::fs::dup(oldfd.as_u32()) {
            Ok(newfd) => {
                fs_io::linux_fd_clear_descriptor_flags(newfd);
                newfd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_dup2(oldfd: Fd, newfd: Fd) -> usize {
    crate::require_posix_fs!((oldfd, newfd) => {
        match crate::modules::posix::fs::dup2(oldfd.as_u32(), newfd.as_u32()) {
            Ok(fd) => {
                fs_io::linux_fd_clear_descriptor_flags(fd);
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_dup3(oldfd: Fd, newfd: Fd, flags: usize) -> usize {
    if oldfd.as_i32() == newfd.as_i32() {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !linux::open_flags::O_CLOEXEC) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    crate::require_posix_fs!((oldfd, newfd, flags) => {
        match crate::modules::posix::fs::dup2(oldfd.as_u32(), newfd.as_u32()) {
            Ok(fd) => {
                if (flags & linux::open_flags::O_CLOEXEC) != 0 {
                    fs_io::linux_fd_set_descriptor_flags(fd, fs_io::LINUX_FD_CLOEXEC);
                } else {
                    fs_io::linux_fd_clear_descriptor_flags(fd);
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}