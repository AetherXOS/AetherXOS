use super::*;

#[cfg(feature = "network_transport")]
pub fn getsockname(fd: u32) -> Result<SocketAddrV4, &'static str> {
    ensure_posix_available()?;

    with_socket(fd, |s| {
        let inner = s.inner.lock();
        match &*inner {
            PosixSocket::Datagram(state) => Ok(SocketAddrV4::localhost(state.local_port.unwrap_or(0))),
            PosixSocket::Stream(StreamState::Unbound { local_port }) => {
                Ok(SocketAddrV4::localhost(local_port.unwrap_or(0)))
            }
            PosixSocket::Stream(StreamState::Listening { listener, .. }) => {
                Ok(SocketAddrV4::localhost(listener.local_port()))
            }
            PosixSocket::Stream(StreamState::Connected { stream, .. }) => {
                Ok(SocketAddrV4::localhost(stream.local_port()))
            }
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn getpeername(fd: u32) -> Result<SocketAddrV4, &'static str> {
    ensure_posix_available()?;

    with_socket(fd, |s| {
        let inner = s.inner.lock();
        match &*inner {
            PosixSocket::Datagram(state) => Ok(SocketAddrV4::localhost(state.peer_port.unwrap_or(0))),
            PosixSocket::Stream(StreamState::Connected { stream, .. }) => {
                Ok(SocketAddrV4::localhost(stream.peer_port()))
            }
            PosixSocket::Stream(StreamState::Listening { .. }) => {
                Err("listening socket has no peer")
            }
            PosixSocket::Stream(StreamState::Unbound { .. }) => Err("unbound socket has no peer"),
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn shutdown(fd: u32, how: ShutdownHow) -> Result<(), &'static str> {
    ensure_posix_available()?;
    with_socket(fd, |s| {
        let mut flags = s.flags.lock();
        apply_shutdown(&mut flags, how);
        Ok(())
    })
}

#[cfg(feature = "network_transport")]
pub fn close(fd: u32) -> Result<(), &'static str> {
    crate::modules::posix::fs::FILE_TABLE
        .lock()
        .remove(&fd)
        .ok_or("invalid socket fd")?;
    Ok(())
}