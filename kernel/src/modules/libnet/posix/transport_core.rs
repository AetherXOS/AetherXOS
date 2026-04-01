use super::*;

#[cfg(feature = "network_transport")]
pub fn socket(family: AddressFamily, ty: SocketType) -> Result<u32, &'static str> {
    ensure_posix_available()?;
    if family != AddressFamily::Inet {
        return Err("unsupported address family");
    }

    let entry = match ty {
        SocketType::Datagram => PosixSocket::Datagram(DatagramState {
            socket: None,
            local_port: None,
            peer_port: None,
            pending: VecDeque::new(),
        }),
        SocketType::Stream => PosixSocket::Stream(StreamState::Unbound { local_port: None }),
    };

    crate::modules::posix::fs::register_posix_handle(Arc::new(Mutex::new(SocketFile {
        inner: Arc::new(Mutex::new(entry)),
        flags: Arc::new(Mutex::new(SocketFlags::for_socket(family, ty))),
    })))
    .map_err(|_| "fd registration failed")
}

#[cfg(feature = "network_transport")]
pub fn bind(fd: u32, addr: SocketAddrV4) -> Result<(), &'static str> {
    ensure_posix_available()?;
    if addr.port == 0 {
        return Err("invalid bind port");
    }

    with_socket_mut(fd, |s| {
        let mut inner = s.inner.lock();
        match &mut *inner {
            PosixSocket::Datagram(state) => {
                if state.socket.is_some() {
                    return Err("datagram socket already bound");
                }
                state.socket = Some(crate::modules::libnet::udp_bind(addr.port)?);
                state.local_port = Some(addr.port);
                Ok(())
            }
            PosixSocket::Stream(StreamState::Unbound { local_port }) => {
                *local_port = Some(addr.port);
                Ok(())
            }
            PosixSocket::Stream(_) => Err("invalid stream state for bind"),
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn listen(fd: u32, _backlog: usize) -> Result<(), &'static str> {
    ensure_posix_available()?;

    with_socket_mut(fd, |s| {
        let mut inner = s.inner.lock();
        match &mut *inner {
            PosixSocket::Stream(StreamState::Unbound { local_port }) => {
                let port = local_port.ok_or("stream socket must be bound before listen")?;
                let listener = crate::modules::libnet::tcp_listen(port)?;
                *inner = PosixSocket::Stream(StreamState::Listening {
                    listener,
                    pending_accepts: VecDeque::new(),
                });
                Ok(())
            }
            PosixSocket::Stream(StreamState::Listening { .. }) => Ok(()),
            PosixSocket::Stream(StreamState::Connected { .. }) => {
                Err("connected stream cannot listen")
            }
            PosixSocket::Datagram(_) => Err("listen requires stream socket"),
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn connect(fd: u32, addr: SocketAddrV4) -> Result<(), &'static str> {
    ensure_posix_available()?;

    with_socket_mut(fd, |s| {
        let mut inner = s.inner.lock();
        match &mut *inner {
            PosixSocket::Stream(StreamState::Unbound { local_port }) => {
                let local = local_port.unwrap_or_else(alloc_ephemeral_port);
                let stream = crate::modules::libnet::tcp_connect(local, addr.port)?;
                *inner = PosixSocket::Stream(StreamState::Connected {
                    stream,
                    pending: VecDeque::new(),
                });
                Ok(())
            }
            PosixSocket::Stream(StreamState::Connected { .. }) => Ok(()),
            PosixSocket::Stream(StreamState::Listening { .. }) => {
                Err("listening socket cannot connect")
            }
            PosixSocket::Datagram(state) => {
                ensure_datagram_socket(state)?;
                state.peer_port = Some(addr.port);
                Ok(())
            }
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn accept(fd: u32) -> Result<u32, &'static str> {
    ensure_posix_available()?;

    let (nonblocking, retries) = with_socket(fd, |s| {
        let flags = s.flags.lock();
        Ok((flags.nonblocking, flags.recv_timeout_retries))
    })?;

    for _ in 0..=retries {
        let maybe_stream = with_socket_mut(fd, |s| {
            let mut inner = s.inner.lock();
            match &mut *inner {
                PosixSocket::Stream(StreamState::Listening {
                    listener,
                    pending_accepts,
                }) => {
                    if let Some(stream) = pending_accepts.pop_front() {
                        Ok(Some(stream))
                    } else {
                        Ok(listener.accept())
                    }
                }
                _ => Err("accept requires listening stream socket"),
            }
        })?;

        if let Some(stream) = maybe_stream {
            let new_s = SocketFile {
                inner: Arc::new(Mutex::new(PosixSocket::Stream(StreamState::Connected {
                    stream,
                    pending: VecDeque::new(),
                }))),
                flags: Arc::new(Mutex::new(SocketFlags::for_socket(
                    AddressFamily::Inet,
                    SocketType::Stream,
                ))),
            };
            let new_fd =
                crate::modules::posix::fs::register_posix_handle(Arc::new(Mutex::new(new_s)))
                    .map_err(|_| "failed to register fd")?;
            clear_last_error(fd);
            return Ok(new_fd);
        }

        if nonblocking {
            set_last_error(fd, PosixErrno::WouldBlock);
            return Err("would block");
        }
        poll_transport_hint();
    }

    set_last_error(fd, PosixErrno::TimedOut);
    Err("would block")
}


