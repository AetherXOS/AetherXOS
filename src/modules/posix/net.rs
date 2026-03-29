use crate::modules::posix_consts::net_typed as typed_net;

pub use crate::modules::libnet::{
    posix_accept, posix_accept4, posix_bind, posix_close, posix_connect, posix_dup, posix_dup2,
    posix_fcntl, posix_fcntl_setfl_errno, posix_getpeername, posix_getsockname, posix_getsockopt,
    posix_ioctl, posix_listen, posix_poll, posix_recv, posix_recv_with_flags_errno, posix_recvfrom,
    posix_recvfrom_with_flags_errno, posix_select, posix_send, posix_sendto, posix_setsockopt,
    posix_shutdown, posix_socket, PosixAddressFamily, PosixFcntlCmd, PosixFdFlags, PosixIoctlCmd,
    PosixMsgFlags, PosixPollEvents, PosixPollFd, PosixSelectResult, PosixShutdownHow, PosixSockOpt,
    PosixSockOptVal, PosixSocketAddrV4, PosixSocketType,
};

use super::PosixErrno;

#[path = "net/ancillary.rs"]
mod ancillary;
#[path = "net/epoll_support.rs"]
mod epoll_support;
#[path = "net/msg_support.rs"]
mod msg_support;
#[path = "net/socketpair_support.rs"]
mod socketpair_support;
#[path = "net/unix_support.rs"]
mod unix_support;

pub use epoll_support::{
    await_readable, await_writable, epoll_close, epoll_create1, epoll_ctl, epoll_ctl_typed,
    epoll_pwait, epoll_pwait2, epoll_wait, epoll_wait_timeout_ms, EpollCtlOp, EpollEvent,
    EpollTimeout,
};
pub use msg_support::{
    recvfrom_flags, recvmmsg, recvmmsg_async, recvmsg, recvmsg_async, recvmsg_flags, recvmsg_full,
    sendmmsg, sendmmsg_async, sendmsg, sendmsg_async, sendmsg_flags, sendmsg_full,
};
pub use socketpair_support::{socketpair, socketpair_typed};
pub use unix_support::{
    unix_accept, unix_bind_addr, unix_bind_path, unix_bind_sockaddr, unix_connect_addr,
    unix_connect_path, unix_connect_sockaddr, unix_listen, unix_recvfrom_addr, unix_sendto_path,
    unix_sendto_sockaddr, unix_unlink_addr, unix_unlink_path, unix_unlink_sockaddr, SockAddrUn,
};

pub(super) fn map_net_errno(err: crate::modules::libnet::PosixErrno) -> PosixErrno {
    match err {
        crate::modules::libnet::PosixErrno::Again => PosixErrno::Again,
        crate::modules::libnet::PosixErrno::BadFileDescriptor => PosixErrno::BadFileDescriptor,
        crate::modules::libnet::PosixErrno::Invalid => PosixErrno::Invalid,
        crate::modules::libnet::PosixErrno::NotConnected => PosixErrno::NotConnected,
        crate::modules::libnet::PosixErrno::AddrInUse => PosixErrno::AddrInUse,
        crate::modules::libnet::PosixErrno::TimedOut => PosixErrno::TimedOut,
        crate::modules::libnet::PosixErrno::NotSupported => PosixErrno::NotSupported,
        crate::modules::libnet::PosixErrno::WouldBlock => PosixErrno::Again,
        crate::modules::libnet::PosixErrno::Other => PosixErrno::Other,
    }
}

fn decode_socket_type_and_flags(
    raw_type: i32,
) -> Result<(PosixSocketType, PosixFdFlags), PosixErrno> {
    let base = raw_type & crate::modules::posix_consts::net::SOCK_TYPE_MASK;
    let socket_type = PosixSocketType::from_raw(base).ok_or(PosixErrno::Invalid)?;

    let allowed_flags = crate::modules::posix_consts::net::SOCK_NONBLOCK
        | crate::modules::posix_consts::net::SOCK_CLOEXEC
        | crate::modules::posix_consts::net::SOCK_TYPE_MASK;
    if (raw_type & !allowed_flags) != 0 {
        return Err(PosixErrno::Invalid);
    }

    let mut fd_flags = PosixFdFlags::empty();
    if (raw_type & crate::modules::posix_consts::net::SOCK_NONBLOCK) != 0 {
        fd_flags.insert(PosixFdFlags::NONBLOCK);
    }
    Ok((socket_type, fd_flags))
}

fn decode_family_raw(family_raw: i32) -> Result<PosixAddressFamily, PosixErrno> {
    match family_raw {
        crate::modules::posix_consts::net::AF_INET
        | crate::modules::posix_consts::net::AF_UNSPEC
        | crate::modules::posix_consts::net::AF_UNIX => Ok(PosixAddressFamily::Inet),
        _ => Err(PosixErrno::Invalid),
    }
}

fn decode_sockopt_raw(opt_raw: i32) -> Result<PosixSockOpt, PosixErrno> {
    PosixSockOpt::from_raw(opt_raw).ok_or(PosixErrno::Invalid)
}

fn validate_sockopt_level(level_raw: i32) -> Result<(), PosixErrno> {
    if level_raw != crate::modules::posix_consts::net::SOL_SOCKET {
        return Err(PosixErrno::Invalid);
    }
    Ok(())
}

fn decode_accept4_flags_raw(flags_raw: i32) -> Result<PosixFdFlags, PosixErrno> {
    let allowed = crate::modules::posix_consts::net::SOCK_NONBLOCK
        | crate::modules::posix_consts::net::SOCK_CLOEXEC;
    if (flags_raw & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }

    let mut out = PosixFdFlags::empty();
    if (flags_raw & crate::modules::posix_consts::net::SOCK_NONBLOCK) != 0 {
        out.insert(PosixFdFlags::NONBLOCK);
    }
    Ok(out)
}

fn validate_protocol(socket_type: PosixSocketType, protocol: i32) -> Result<(), PosixErrno> {
    use crate::modules::posix_consts::net::{IPPROTO_IP, IPPROTO_TCP, IPPROTO_UDP};

    if protocol == 0 || protocol == IPPROTO_IP {
        return Ok(());
    }

    match socket_type {
        PosixSocketType::Stream if protocol == IPPROTO_TCP => Ok(()),
        PosixSocketType::Datagram if protocol == IPPROTO_UDP => Ok(()),
        _ => Err(PosixErrno::Invalid),
    }
}

pub fn socket_raw_errno(
    family_raw: i32,
    socket_type_raw: i32,
    protocol: i32,
) -> Result<u32, PosixErrno> {
    let family = decode_family_raw(family_raw)?;
    let (socket_type, fd_flags) = decode_socket_type_and_flags(socket_type_raw)?;
    validate_protocol(socket_type, protocol)?;
    let fd = socket_errno(family, socket_type)?;
    if !fd_flags.is_empty() {
        posix_fcntl_setfl_errno(fd, fd_flags).map_err(map_net_errno)?;
    }
    Ok(fd)
}

#[inline(always)]
pub fn socket_typed_errno(
    family: typed_net::AddressFamily,
    socket_type: typed_net::SocketType,
    protocol: typed_net::Protocol,
) -> Result<u32, PosixErrno> {
    socket_raw_errno(family.as_raw(), socket_type.as_raw(), protocol.as_raw())
}

pub fn socketpair_raw_errno(
    domain_raw: i32,
    socket_type_raw: i32,
    protocol: i32,
) -> Result<(u32, u32), PosixErrno> {
    match domain_raw {
        crate::modules::posix_consts::net::AF_UNIX
        | crate::modules::posix_consts::net::AF_UNSPEC
        | crate::modules::posix_consts::net::AF_INET => {}
        _ => return Err(PosixErrno::Invalid),
    }

    let (socket_type, fd_flags) = decode_socket_type_and_flags(socket_type_raw)?;
    validate_protocol(socket_type, protocol)?;
    let (fd0, fd1) = socketpair_typed(socket_type)?;
    if !fd_flags.is_empty() {
        if let Err(err) = posix_fcntl_setfl_errno(fd0, fd_flags).map_err(map_net_errno) {
            let _ = close(fd0);
            let _ = close(fd1);
            return Err(err);
        }
        if let Err(err) = posix_fcntl_setfl_errno(fd1, fd_flags).map_err(map_net_errno) {
            let _ = close(fd0);
            let _ = close(fd1);
            return Err(err);
        }
    }
    Ok((fd0, fd1))
}

#[inline(always)]
pub fn socketpair_typed_errno(
    domain: typed_net::AddressFamily,
    socket_type: typed_net::SocketType,
    protocol: typed_net::Protocol,
) -> Result<(u32, u32), PosixErrno> {
    socketpair_raw_errno(domain.as_raw(), socket_type.as_raw(), protocol.as_raw())
}

pub fn accept4_raw_errno(fd: u32, flags_raw: i32) -> Result<u32, PosixErrno> {
    let flags = decode_accept4_flags_raw(flags_raw)?;
    crate::modules::libnet::posix_accept4_errno(fd, flags).map_err(map_net_errno)
}

pub fn setsockopt_raw(fd: u32, level_raw: i32, opt_raw: i32, value: u64) -> Result<(), PosixErrno> {
    validate_sockopt_level(level_raw)?;
    let opt = decode_sockopt_raw(opt_raw)?;

    let val = match opt {
        PosixSockOpt::SocketType | PosixSockOpt::SocketDomain | PosixSockOpt::SocketError => {
            return Err(PosixErrno::Invalid)
        }
        PosixSockOpt::NonBlocking | PosixSockOpt::ReuseAddr => PosixSockOptVal::Bool(value != 0),
        PosixSockOpt::RecvTimeout
        | PosixSockOpt::SendTimeout
        | PosixSockOpt::RecvTimeoutRetries
        | PosixSockOpt::SendTimeoutRetries => PosixSockOptVal::Usize(value as usize),
    };

    crate::modules::libnet::posix_setsockopt_errno(fd, opt, val).map_err(map_net_errno)
}

pub fn getsockopt_raw(fd: u32, level_raw: i32, opt_raw: i32) -> Result<u64, PosixErrno> {
    validate_sockopt_level(level_raw)?;
    let opt = decode_sockopt_raw(opt_raw)?;
    let value = crate::modules::libnet::posix_getsockopt_errno(fd, opt).map_err(map_net_errno)?;
    match value {
        PosixSockOptVal::Bool(v) => Ok(if v { 1 } else { 0 }),
        PosixSockOptVal::Usize(v) => Ok(v as u64),
        PosixSockOptVal::Errno(e) => Ok(e.code() as u64),
    }
}

pub fn socket_errno(
    family: PosixAddressFamily,
    socket_type: PosixSocketType,
) -> Result<u32, PosixErrno> {
    crate::modules::libnet::posix_socket_errno(family, socket_type).map_err(map_net_errno)
}

pub fn close(fd: u32) -> Result<(), PosixErrno> {
    crate::modules::libnet::posix_close_errno(fd).map_err(map_net_errno)?;
    unix_support::on_close_fd(fd);
    epoll_support::on_close_fd(fd);
    Ok(())
}
