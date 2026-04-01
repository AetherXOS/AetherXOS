use super::super::super::*;

#[cfg(not(feature = "linux_compat"))]
#[allow(unreachable_code)]
pub(crate) fn sys_linux_read(fd: usize, ptr: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if is_stdio_fd(fd) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }

    #[cfg(feature = "posix_fs")]
    {
        if fd == STDIN_FD {
            return 0;
        }

        let fs_result =
            with_user_write_bytes(ptr, len, |dst| {
                match crate::modules::posix::fs::read(fd as u32, dst) {
                    Ok(n) => n,
                    Err(err) => linux_errno(err.code()),
                }
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
        if fs_result != linux_errno(crate::modules::posix_consts::errno::EBADF) {
            return fs_result;
        }
    }

    #[cfg(feature = "posix_net")]
    {
        return with_user_write_bytes(
            ptr,
            len,
            |dst| match crate::modules::libnet::posix_recv_errno(fd as u32) {
                Ok(packet) => {
                    let copy_len = core::cmp::min(dst.len(), packet.len());
                    dst[..copy_len].copy_from_slice(&packet[..copy_len]);
                    copy_len
                }
                Err(err) => linux_errno(err.code()),
            },
        )
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = (fd, ptr, len);
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }

    #[cfg(all(feature = "posix_fs", not(feature = "posix_net")))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(all(not(feature = "posix_fs"), feature = "posix_net"))]
    {
        unreachable!("posix_net path returns above")
    }

    #[cfg(all(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
#[allow(unreachable_code)]
pub(crate) fn sys_linux_write(fd: usize, ptr: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if !is_stdio_fd(fd) {
        #[cfg(feature = "posix_fs")]
        {
            let fs_result = with_user_read_bytes(ptr, len, |src| {
                match crate::modules::posix::fs::write(fd as u32, src) {
                    Ok(n) => n,
                    Err(err) => linux_errno(err.code()),
                }
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
            if fs_result != linux_errno(crate::modules::posix_consts::errno::EBADF) {
                return fs_result;
            }
        }
        #[cfg(feature = "posix_net")]
        {
            return with_user_read_bytes(ptr, len, |src| {
                match crate::modules::libnet::posix_send_errno(fd as u32, src) {
                    Ok(n) => n,
                    Err(err) => linux_errno(err.code()),
                }
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
        }
        #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
        {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
        #[cfg(all(feature = "posix_fs", not(feature = "posix_net")))]
        {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
        #[cfg(all(not(feature = "posix_fs"), feature = "posix_net"))]
        {
            unreachable!("posix_net path returns above");
        }
        #[cfg(all(feature = "posix_fs", feature = "posix_net"))]
        {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
    }

    with_user_read_bounded_bytes(ptr, len, MAX_PRINT_LEN, |slice| {
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::klog_info!("LINUX: {}", s);
            slice.len()
        } else {
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        }
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_close(fd: usize) -> usize {
    if fd <= STDERR_FD {
        return 0;
    }

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::close(fd as u32) {
            Ok(()) => {
                super::super::super::fd_process_identity::clear_getdents_cursor(fd as u32);
                super::super::super::fd_process_identity::clear_linux_fd_flags(fd as u32);
                super::super::super::fd_process_identity::clear_linux_pidfd_entry(fd as u32);
                return 0;
            }
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }

    #[cfg(feature = "posix_net")]
    {
        match crate::modules::posix::net::close(fd as u32) {
            Ok(()) => {
                super::super::super::fd_process_identity::clear_getdents_cursor(fd as u32);
                super::super::super::fd_process_identity::clear_linux_fd_flags(fd as u32);
                super::super::super::fd_process_identity::clear_linux_pidfd_entry(fd as u32);
                return 0;
            }
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }

    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_lseek(fd: usize, offset: i64, whence_raw: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let whence = match whence_raw {
            SEEK_SET => crate::modules::posix::fs::SeekWhence::Set,
            SEEK_CUR => crate::modules::posix::fs::SeekWhence::Cur,
            SEEK_END => crate::modules::posix::fs::SeekWhence::End,
            _ => return linux_errno(crate::modules::posix_consts::errno::EINVAL),
        };

        match crate::modules::posix::fs::lseek(fd as u32, offset, whence) {
            Ok(pos) => pos as usize,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, offset, whence_raw);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn stdout_write_invalid_ptr_returns_efault() {
        assert_eq!(
            sys_linux_write(STDOUT_FD, 0, 1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn non_stdio_write_unknown_fd_returns_ebadf_when_backends_exist() {
        #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
        {
            let payload = b"x";
            assert_eq!(
                sys_linux_write(usize::MAX, payload.as_ptr() as usize, payload.len()),
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            );
        }
    }
}
