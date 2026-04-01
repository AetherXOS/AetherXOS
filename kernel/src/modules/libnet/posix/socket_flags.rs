use super::{
    AddressFamily, PosixErrno, PosixFdFlags, PosixSockOpt, PosixSockOptVal, PosixSocketOptions,
    ShutdownHow, SocketType,
};

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub(super) struct SocketFlags {
    pub(super) family: AddressFamily,
    pub(super) socket_type: SocketType,
    pub(super) nonblocking: bool,
    pub(super) reuse_addr: bool,
    pub(super) shutdown_read: bool,
    pub(super) shutdown_write: bool,
    pub(super) recv_timeout_retries: usize,
    pub(super) send_timeout_retries: usize,
    pub(super) last_error: Option<PosixErrno>,
}

#[cfg(feature = "network_transport")]
impl SocketFlags {
    pub(super) fn for_socket(family: AddressFamily, socket_type: SocketType) -> Self {
        let blocking_recv_retries =
            crate::config::KernelConfig::libnet_posix_blocking_recv_retries();
        Self {
            family,
            socket_type,
            nonblocking: false,
            reuse_addr: false,
            shutdown_read: false,
            shutdown_write: false,
            recv_timeout_retries: blocking_recv_retries,
            send_timeout_retries: blocking_recv_retries,
            last_error: None,
        }
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn apply_socket_option(
    flags: &mut SocketFlags,
    option: PosixSockOpt,
    value: PosixSockOptVal,
) -> Result<(), &'static str> {
    match (option, value) {
        (PosixSockOpt::SocketType, _) => Err("socket type option is read-only"),
        (PosixSockOpt::NonBlocking, PosixSockOptVal::Bool(enabled)) => {
            flags.nonblocking = enabled;
            Ok(())
        }
        (PosixSockOpt::ReuseAddr, PosixSockOptVal::Bool(enabled)) => {
            flags.reuse_addr = enabled;
            Ok(())
        }
        (PosixSockOpt::RecvTimeout, PosixSockOptVal::Usize(retries))
        | (PosixSockOpt::RecvTimeoutRetries, PosixSockOptVal::Usize(retries)) => {
            flags.recv_timeout_retries = retries;
            Ok(())
        }
        (PosixSockOpt::SendTimeout, PosixSockOptVal::Usize(retries))
        | (PosixSockOpt::SendTimeoutRetries, PosixSockOptVal::Usize(retries)) => {
            flags.send_timeout_retries = retries;
            Ok(())
        }
        (PosixSockOpt::SocketDomain, _) => Err("socket domain option is read-only"),
        (PosixSockOpt::SocketError, _) => Err("socket error option is read-only"),
        _ => Err("invalid socket option value"),
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn query_socket_option(flags: SocketFlags, option: PosixSockOpt) -> PosixSockOptVal {
    match option {
        PosixSockOpt::SocketType => PosixSockOptVal::Usize(flags.socket_type.as_raw() as usize),
        PosixSockOpt::NonBlocking => PosixSockOptVal::Bool(flags.nonblocking),
        PosixSockOpt::ReuseAddr => PosixSockOptVal::Bool(flags.reuse_addr),
        PosixSockOpt::RecvTimeout => PosixSockOptVal::Usize(flags.recv_timeout_retries),
        PosixSockOpt::SendTimeout => PosixSockOptVal::Usize(flags.send_timeout_retries),
        PosixSockOpt::RecvTimeoutRetries => PosixSockOptVal::Usize(flags.recv_timeout_retries),
        PosixSockOpt::SendTimeoutRetries => PosixSockOptVal::Usize(flags.send_timeout_retries),
        PosixSockOpt::SocketDomain => PosixSockOptVal::Usize(flags.family.as_raw() as usize),
        PosixSockOpt::SocketError => {
            PosixSockOptVal::Errno(flags.last_error.unwrap_or(PosixErrno::Other))
        }
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn socket_options_from_flags(flags: SocketFlags) -> PosixSocketOptions {
    PosixSocketOptions {
        nonblocking: flags.nonblocking,
        reuse_addr: flags.reuse_addr,
        recv_timeout_retries: flags.recv_timeout_retries,
        send_timeout_retries: flags.send_timeout_retries,
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn fd_flags_from_socket_flags(flags: SocketFlags) -> PosixFdFlags {
    let mut out = PosixFdFlags::empty();
    if flags.nonblocking {
        out.insert(PosixFdFlags::NONBLOCK);
    }
    out
}

#[cfg(feature = "network_transport")]
pub(super) fn apply_fd_flags(flags: &mut SocketFlags, fd_flags: PosixFdFlags) {
    flags.nonblocking = fd_flags.contains(PosixFdFlags::NONBLOCK);
}

#[cfg(feature = "network_transport")]
pub(super) fn apply_shutdown(flags: &mut SocketFlags, how: ShutdownHow) {
    match how {
        ShutdownHow::Read => flags.shutdown_read = true,
        ShutdownHow::Write => flags.shutdown_write = true,
        ShutdownHow::Both => {
            flags.shutdown_read = true;
            flags.shutdown_write = true;
        }
    }
}
