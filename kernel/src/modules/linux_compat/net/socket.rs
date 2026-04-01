use super::super::*;

/// `getsockopt(2)` — Get options on sockets.
pub fn sys_linux_getsockopt(
    fd: Fd,
    level: usize,
    optname: usize,
    optval_ptr: UserPtr<u8>,
    optlen_ptr: UserPtr<u32>,
) -> usize {
    crate::require_posix_net!((fd, level, optname, optval_ptr, optlen_ptr) => {
        let optlen = match optlen_ptr.read() { Ok(v) => v, Err(e) => return e };
        if optlen == 0 { return err::inval(); }

        match crate::modules::posix::net::getsockopt_raw(fd.as_u32(), level as i32, optname as i32) {
            Ok(val) => {
                let to_write = (optlen as usize).min(core::mem::size_of::<u64>());
                let _ = optval_ptr.write_bytes_with(to_write, |dst| {
                    dst.copy_from_slice(&val.to_ne_bytes()[..to_write]);
                    0
                });
                write_user_struct!(optlen_ptr, to_write as u32)
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `setsockopt(2)` — Set options on sockets.
pub fn sys_linux_setsockopt(
    fd: Fd,
    level: usize,
    optname: usize,
    optval_ptr: UserPtr<u8>,
    optlen: usize,
) -> usize {
    crate::require_posix_net!((fd, level, optname, optval_ptr, optlen) => {
        if optlen == 0 { return err::inval(); }

        let value = match optlen {
            4 => match optval_ptr.cast::<u32>().read() { Ok(v) => v as u64, Err(e) => return e },
            8 => match optval_ptr.cast::<u64>().read() { Ok(v) => v, Err(e) => return e },
            _ if optlen >= 4 => match optval_ptr.cast::<u32>().read() { Ok(v) => v as u64, Err(e) => return e },
            _ => 0,
        };

        match crate::modules::posix::net::setsockopt_raw(fd.as_u32(), level as i32, optname as i32, value) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `socket(2)` — Create an endpoint for communication.
pub fn sys_linux_socket(domain: usize, sock_type: usize, protocol: usize) -> usize {
    crate::require_posix_net!((domain, sock_type, protocol) => {
        match crate::modules::posix::net::socket_raw_errno(
            domain as i32,
            sock_type as i32,
            protocol as i32,
        ) {
            Ok(fd) => {
                if (sock_type & linux::open_flags::O_CLOEXEC) != 0 {
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        fd,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                } else {
                    crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(fd);
                }
                linux_trace!(
                    "[NET] New socket: domain={}, type={}, fd={}\n",
                    domain,
                    sock_type,
                    fd
                );
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    })
}

/// `socketpair(2)` — Create a pair of connected sockets.
pub fn sys_linux_socketpair(
    domain: usize,
    sock_type: usize,
    protocol: usize,
    sv_ptr: UserPtr<Fd>,
) -> usize {
    crate::require_posix_net!((domain, sock_type, protocol, sv_ptr) => {
        match crate::modules::posix::net::socketpair_raw_errno(
            domain as i32,
            sock_type as i32,
            protocol as i32,
        ) {
            Ok((f0, f1)) => {
                if (sock_type & linux::open_flags::O_CLOEXEC) != 0 {
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        f0,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        f1,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                } else {
                    crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(f0);
                    crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(f1);
                }
                if let Err(e) = sv_ptr.write(&Fd(f0 as i32)) { return e; }
                write_user_struct!(sv_ptr.add(1), Fd(f1 as i32))
            }
            Err(err) => linux_errno(err.code()),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn socket_with_cloexec_sets_linux_descriptor_flag() {
        let fd = sys_linux_socket(
            crate::modules::posix_consts::net::AF_UNIX as usize,
            (crate::modules::posix_consts::net::SOCK_STREAM
                | crate::modules::posix_consts::net::SOCK_CLOEXEC) as usize,
            0,
        ) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
    }

    #[test_case]
    fn socketpair_with_cloexec_sets_linux_descriptor_flags() {
        let mut fds = [Fd(-1), Fd(-1)];
        let result = sys_linux_socketpair(
            crate::modules::posix_consts::net::AF_UNIX as usize,
            (crate::modules::posix_consts::net::SOCK_STREAM
                | crate::modules::posix_consts::net::SOCK_CLOEXEC) as usize,
            0,
            UserPtr::new(fds.as_mut_ptr() as usize),
        );
        assert_eq!(result, 0);
        for fd in fds {
            assert_eq!(
                crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd.as_u32())
                    & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
            );
        }
    }
}
