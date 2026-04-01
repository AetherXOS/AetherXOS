use super::*;

#[cfg(feature = "network_transport")]
pub fn set_nonblocking(fd: u32, _enabled: bool) -> Result<(), &'static str> {
    ensure_posix_available()?;
    set_socket_flags(fd, |flags| {
        flags.nonblocking = _enabled;
    })
}

#[cfg(feature = "network_transport")]
pub fn set_socket_option(fd: u32, option: SocketOption, enabled: bool) -> Result<(), &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| {
        let mut flags = s.flags.lock();
        match option {
            SocketOption::NonBlocking => flags.nonblocking = enabled,
            SocketOption::ReuseAddr => flags.reuse_addr = enabled,
        }
        Ok(())
    })
}

#[cfg(feature = "network_transport")]
pub fn setsockopt(
    fd: u32,
    option: PosixSockOpt,
    value: PosixSockOptVal,
) -> Result<(), &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| {
        let mut flags = s.flags.lock();
        apply_socket_option(&mut flags, option, value)
    })
}

#[cfg(feature = "network_transport")]
pub fn getsockopt(fd: u32, option: PosixSockOpt) -> Result<PosixSockOptVal, &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| Ok(query_socket_option(*s.flags.lock(), option)))
}

#[cfg(feature = "network_transport")]
pub fn dup(fd: u32) -> Result<u32, &'static str> {
    crate::modules::posix::fs::dup(fd).map_err(|_| "dup failed")
}

#[cfg(feature = "network_transport")]
pub fn dup2(oldfd: u32, newfd: u32) -> Result<u32, &'static str> {
    crate::modules::posix::fs::dup2(oldfd, newfd).map_err(|_| "dup2 failed")
}

#[cfg(feature = "network_transport")]
pub fn accept4(fd: u32, flags: PosixFdFlags) -> Result<u32, &'static str> {
    let new_fd = accept(fd)?;
    fcntl_setfl(new_fd, flags)?;
    Ok(new_fd)
}

#[cfg(feature = "network_transport")]
pub fn ioctl(fd: u32, cmd: PosixIoctlCmd) -> Result<usize, &'static str> {
    ensure_posix_available()?;
    match cmd {
        PosixIoctlCmd::FionRead => with_socket(fd, |s| {
            let inner = s.inner.lock();
            match &*inner {
                PosixSocket::Datagram(state) => {
                    Ok(state.pending.iter().map(|d| d.payload.len()).sum())
                }
                PosixSocket::Stream(StreamState::Unbound { .. }) => Ok(0),
                PosixSocket::Stream(StreamState::Listening {
                    pending_accepts, ..
                }) => Ok(pending_accepts.len()),
                PosixSocket::Stream(StreamState::Connected { pending, .. }) => {
                    Ok(pending.iter().map(|chunk| chunk.len()).sum())
                }
            }
        }),
    }
}

#[cfg(feature = "network_transport")]
pub fn socket_options(fd: u32) -> Result<PosixSocketOptions, &'static str> {
    ensure_posix_available()?;
    Ok(socket_options_from_flags(socket_flags(fd)?))
}

#[cfg(feature = "network_transport")]
pub fn fcntl(fd: u32, cmd: FcntlCmd) -> Result<PosixFdFlags, &'static str> {
    ensure_posix_available()?;
    let _ = socket_flags(fd)?;

    match cmd {
        FcntlCmd::GetFl => fcntl_getfl(fd),
        FcntlCmd::SetFl(flags) => {
            fcntl_setfl(fd, flags)?;
            fcntl_getfl(fd)
        }
    }
}

#[cfg(feature = "network_transport")]
pub fn fcntl_getfl(fd: u32) -> Result<PosixFdFlags, &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| Ok(fd_flags_from_socket_flags(*s.flags.lock())))
}

#[cfg(feature = "network_transport")]
pub fn fcntl_setfl(fd: u32, flags: PosixFdFlags) -> Result<(), &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| {
        let mut socket_flags = s.flags.lock();
        apply_fd_flags(&mut socket_flags, flags);
        Ok(())
    })
}