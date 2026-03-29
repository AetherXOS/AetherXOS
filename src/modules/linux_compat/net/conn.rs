use super::super::*;

/// `accept(2)` — Accept a connection on a socket.
pub fn sys_linux_accept(fd: Fd, addr_ptr: UserPtr<u8>, len_ptr: UserPtr<u32>) -> usize {
    sys_linux_accept4(fd, addr_ptr, len_ptr, 0)
}

/// `accept4(2)` — Accept a connection on a socket with flags.
pub fn sys_linux_accept4(
    fd: Fd,
    addr_ptr: UserPtr<u8>,
    len_ptr: UserPtr<u32>,
    flags_raw: usize,
) -> usize {
    crate::require_posix_net!((fd, addr_ptr, len_ptr, flags_raw) => {
        let fd_u32 = fd.as_u32();
        let accepted_fd = if flags_raw == 0 {
            crate::modules::libnet::posix_accept_errno(fd_u32).map_err(|e| e as i32)
        } else {
            crate::modules::posix::net::accept4_raw_errno(fd_u32, flags_raw as i32).map_err(|e| e.code())
        };

        let new_fd = match accepted_fd {
            Ok(v) => v,
            Err(code) => return linux_errno(code),
        };

        if (flags_raw & (crate::modules::posix_consts::net::SOCK_CLOEXEC as usize)) != 0 {
            crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                new_fd,
                crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            );
        } else {
            crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(new_fd);
        }

        if !addr_ptr.is_null() && !len_ptr.is_null() {
            if let Ok(peer) = crate::modules::libnet::posix_getpeername_errno(new_fd) {
                let _ = write_sockaddr_in(addr_ptr.addr, len_ptr.addr, peer);
            }
        }

        new_fd as usize
    })
}

/// `bind(2)` — Bind a name to a socket.
pub fn sys_linux_bind(fd: Fd, addr_ptr: UserPtr<u8>, addr_len: usize) -> usize {
    crate::require_posix_net!((fd, addr_ptr, addr_len) => {
        let addr = match read_sockaddr_in(addr_ptr.addr, addr_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match crate::modules::libnet::posix_bind_errno(fd.as_u32(), addr) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    })
}

/// `connect(2)` — Initiate a connection on a socket.
pub fn sys_linux_connect(fd: Fd, addr_ptr: UserPtr<u8>, addr_len: usize) -> usize {
    crate::require_posix_net!((fd, addr_ptr, addr_len) => {
        let addr = match read_sockaddr_in(addr_ptr.addr, addr_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match crate::modules::libnet::posix_connect_errno(fd.as_u32(), addr) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    })
}

/// `getpeername(2)` — Get name of connected peer socket.
pub fn sys_linux_getpeername(fd: Fd, addr_ptr: UserPtr<u8>, len_ptr: UserPtr<u32>) -> usize {
    crate::require_posix_net!((fd, addr_ptr, len_ptr) => {
        let addr = match crate::modules::libnet::posix_getpeername_errno(fd.as_u32()) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_sockaddr_in(addr_ptr.addr, len_ptr.addr, addr)
    })
}

/// `getsockname(2)` — Get socket name.
pub fn sys_linux_getsockname(fd: Fd, addr_ptr: UserPtr<u8>, len_ptr: UserPtr<u32>) -> usize {
    crate::require_posix_net!((fd, addr_ptr, len_ptr) => {
        let addr = match crate::modules::libnet::posix_getsockname_errno(fd.as_u32()) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_sockaddr_in(addr_ptr.addr, len_ptr.addr, addr)
    })
}

/// `listen(2)` — Listen for connections on a socket.
pub fn sys_linux_listen(fd: Fd, backlog: usize) -> usize {
    crate::require_posix_net!((fd, backlog) => {
        match crate::modules::libnet::posix_listen_errno(fd.as_u32(), backlog) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    })
}
