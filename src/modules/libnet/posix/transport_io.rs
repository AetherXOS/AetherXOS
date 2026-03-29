use super::*;

#[cfg(feature = "network_transport")]
pub fn send(fd: u32, payload: &[u8]) -> Result<usize, &'static str> {
    ensure_posix_available()?;
    if socket_flags(fd)?.shutdown_write {
        set_last_error(fd, PosixErrno::NotConnected);
        return Err("socket write side is shut down");
    }

    let res = with_socket_mut(fd, |s| {
        let mut inner = s.inner.lock();
        match &mut *inner {
            PosixSocket::Stream(StreamState::Connected { stream, .. }) => stream.send(payload),
            PosixSocket::Datagram(state) => {
                ensure_datagram_socket(state)?;
                let dst = state.peer_port.ok_or("datagram peer not connected")?;
                state
                    .socket
                    .as_ref()
                    .ok_or("datagram socket unavailable")?
                    .send_to(dst, payload)
            }
            _ => Err("socket not connected"),
        }
    });

    if let Err(err) = res {
        set_last_error(fd, map_errno(err));
    } else {
        clear_last_error(fd);
    }
    res
}

#[cfg(feature = "network_transport")]
pub fn recv(fd: u32) -> Result<Vec<u8>, &'static str> {
    recv_with_flags(fd, PosixMsgFlags::empty())
}

#[cfg(feature = "network_transport")]
pub fn recv_with_flags(fd: u32, msg_flags: PosixMsgFlags) -> Result<Vec<u8>, &'static str> {
    ensure_posix_available()?;
    let flags = socket_flags(fd)?;
    if flags.shutdown_read {
        set_last_error(fd, PosixErrno::NotConnected);
        return Err("socket read side is shut down");
    }

    let (nonblocking, retries) = with_socket(fd, |s| {
        let flags = s.flags.lock();
        Ok((
            flags.nonblocking || msg_flags.contains(PosixMsgFlags::DONTWAIT),
            flags.recv_timeout_retries,
        ))
    })?;

    for _ in 0..=retries {
        let maybe_data = with_socket_mut(fd, |s| {
            let mut inner = s.inner.lock();
            match &mut *inner {
                PosixSocket::Stream(StreamState::Connected { stream, pending }) => {
                    let flags = s.flags.lock();
                    if flags.shutdown_read {
                        return Err("socket shutdown for read");
                    }
                    if let Some(data) = pending.pop_front() {
                        if msg_flags.contains(PosixMsgFlags::PEEK) {
                            pending.push_front(data.clone());
                        }
                        return Ok(Some(data));
                    }
                    if let Some(data) = stream.recv() {
                        if msg_flags.contains(PosixMsgFlags::PEEK) {
                            pending.push_front(data.clone());
                        }
                        Ok(Some(data))
                    } else {
                        Ok(None)
                    }
                }
                PosixSocket::Datagram(state) => {
                    let flags = s.flags.lock();
                    if flags.shutdown_read {
                        return Err("socket shutdown for read");
                    }
                    ensure_datagram_socket(state)?;
                    if let Some(dg) = state.pending.pop_front() {
                        if msg_flags.contains(PosixMsgFlags::PEEK) {
                            state.pending.push_front(dg.clone());
                        }
                        return Ok(Some(dg.payload));
                    }
                    if let Some(dg) = state.socket.as_mut().unwrap().recv() {
                        if msg_flags.contains(PosixMsgFlags::PEEK) {
                            state.pending.push_front(dg.clone());
                        }
                        Ok(Some(dg.payload))
                    } else {
                        Ok(None)
                    }
                }
                _ => Err("recv requires connected stream or datagram socket"),
            }
        })?;

        if let Some(data) = maybe_data {
            clear_last_error(fd);
            return Ok(data);
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

#[cfg(feature = "network_transport")]
pub fn sendto(fd: u32, addr: SocketAddrV4, payload: &[u8]) -> Result<usize, &'static str> {
    ensure_posix_available()?;
    with_socket_mut(fd, |s| {
        let mut inner = s.inner.lock();
        match &mut *inner {
            PosixSocket::Datagram(state) => {
                let flags = s.flags.lock();
                if flags.shutdown_write {
                    return Err("socket shutdown for write");
                }
                ensure_datagram_socket(state)?;
                state.socket.as_mut().unwrap().send_to(addr.port, payload)
            }
            PosixSocket::Stream(_) => Err("sendto not supported for stream socket, use send"),
        }
    })
}

#[cfg(feature = "network_transport")]
pub fn recvfrom(fd: u32) -> Result<PosixRecvFrom, &'static str> {
    recvfrom_with_flags(fd, PosixMsgFlags::empty())
}

#[cfg(feature = "network_transport")]
pub fn recvfrom_with_flags(
    fd: u32,
    msg_flags: PosixMsgFlags,
) -> Result<PosixRecvFrom, &'static str> {
    ensure_posix_available()?;
    let flags = socket_flags(fd)?;
    if flags.shutdown_read {
        set_last_error(fd, PosixErrno::NotConnected);
        return Err("socket read side is shut down");
    }

    let peek = msg_flags.contains(PosixMsgFlags::PEEK);
    let (nonblocking, retries) = with_socket(fd, |s| {
        let flags = s.flags.lock();
        Ok((
            flags.nonblocking || msg_flags.contains(PosixMsgFlags::DONTWAIT),
            flags.recv_timeout_retries,
        ))
    })?;

    for _ in 0..=retries {
        let maybe_result = with_socket_mut(fd, |s| {
            let mut inner = s.inner.lock();
            match &mut *inner {
                PosixSocket::Datagram(state) => {
                    let flags = s.flags.lock();
                    if flags.shutdown_read {
                        return Err("socket shutdown for read");
                    }
                    ensure_datagram_socket(state)?;

                    if let Some(datagram) = state.pending.pop_front() {
                        if peek {
                            state.pending.push_front(datagram.clone());
                        }
                        return Ok(Some(PosixRecvFrom {
                            addr: SocketAddrV4::localhost(datagram.src_port),
                            payload: datagram.payload,
                        }));
                    }

                    if let Some(datagram) = state
                        .socket
                        .as_mut()
                        .ok_or("datagram socket unavailable")?
                        .recv()
                    {
                        if peek {
                            state.pending.push_front(datagram.clone());
                        }
                        Ok(Some(PosixRecvFrom {
                            addr: SocketAddrV4::localhost(datagram.src_port),
                            payload: datagram.payload,
                        }))
                    } else {
                        Ok(None)
                    }
                }
                PosixSocket::Stream(StreamState::Connected { stream, pending }) => {
                    let flags = s.flags.lock();
                    if flags.shutdown_read {
                        return Err("socket shutdown for read");
                    }

                    if peek {
                        if let Some(payload) = pending.front() {
                            return Ok(Some(PosixRecvFrom {
                                addr: SocketAddrV4::localhost(stream.peer_port()),
                                payload: payload.clone(),
                            }));
                        }
                        if let Some(payload) = stream.recv() {
                            pending.push_back(payload.clone());
                            return Ok(Some(PosixRecvFrom {
                                addr: SocketAddrV4::localhost(stream.peer_port()),
                                payload,
                            }));
                        }
                        Ok(None)
                    } else if let Some(payload) = pending.pop_front() {
                        Ok(Some(PosixRecvFrom {
                            addr: SocketAddrV4::localhost(stream.peer_port()),
                            payload,
                        }))
                    } else {
                        Ok(stream.recv().map(|payload| PosixRecvFrom {
                            addr: SocketAddrV4::localhost(stream.peer_port()),
                            payload,
                        }))
                    }
                }
                _ => Err("recvfrom requires datagram or connected stream socket"),
            }
        })?;

        if let Some(packet) = maybe_result {
            clear_last_error(fd);
            return Ok(packet);
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