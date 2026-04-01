use super::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

lazy_static::lazy_static! {
    static ref UNIX_REGISTRY: Mutex<BTreeMap<String, Arc<UnixSocket>>> =
        Mutex::new(BTreeMap::new());
}

/// Bind a socket to a path or abstract name.
pub fn unix_bind(path: &str, socket: &Arc<UnixSocket>) -> KernelResult<()> {
    let mut reg = UNIX_REGISTRY.lock();
    if reg.contains_key(path) {
        return Err(KernelError::AlreadyExists);
    }

    let mut inner = socket.inner.lock();
    if inner.state != SocketState::Unbound {
        return Err(KernelError::InvalidInput);
    }

    inner.local_addr = UnixAddr::Path(path.to_string());
    inner.state = SocketState::Bound;
    drop(inner);

    reg.insert(path.to_string(), socket.clone());
    Ok(())
}

/// Set a socket to listening state.
pub fn unix_listen(socket: &Arc<UnixSocket>, backlog: usize) -> KernelResult<()> {
    let mut inner = socket.inner.lock();
    if inner.socket_type != UnixSocketType::Stream {
        return Err(KernelError::InvalidInput);
    }
    if inner.state != SocketState::Bound {
        return Err(KernelError::InvalidInput);
    }
    inner.state = SocketState::Listening;
    inner.backlog = backlog.clamp(1, MAX_BACKLOG);
    Ok(())
}

/// Connect to a listening socket at the given path.
pub fn unix_connect(path: &str) -> KernelResult<Arc<UnixSocket>> {
    let reg = UNIX_REGISTRY.lock();
    let listener = reg.get(path).ok_or(KernelError::NotFound)?.clone();
    drop(reg);

    let client = UnixSocket::new(listener.socket_type());

    let mut l_inner = listener.inner.lock();
    if l_inner.state != SocketState::Listening {
        return Err(KernelError::Disconnected);
    }
    if l_inner.accept_queue.len() >= l_inner.backlog {
        return Err(KernelError::Busy);
    }

    // For immediate accept: create the connection now.
    let capacity = l_inner.rcvbuf;
    let stype = l_inner.socket_type;
    let pair = Arc::new(Mutex::new(PairBuffer {
        a_to_b: if stype == UnixSocketType::Stream {
            ChannelBuf::new_stream(capacity)
        } else {
            ChannelBuf::new_messages()
        },
        b_to_a: if stype == UnixSocketType::Stream {
            ChannelBuf::new_stream(capacity)
        } else {
            ChannelBuf::new_messages()
        },
        a_shut_wr: false,
        b_shut_wr: false,
    }));

    // The accepted server-side socket.
    let server_side = UnixSocket::new(stype);
    {
        let mut s = server_side.inner.lock();
        s.state = SocketState::Connected;
        s.pair = Some(pair.clone());
        s.is_side_a = true;
        s.local_addr = l_inner.local_addr.clone();
        s.peer_addr = Some(UnixAddr::Unnamed);
    }
    l_inner.accept_queue.push_back(server_side);
    listener.rx_wait.wake_all();
    drop(l_inner);

    // Client side.
    {
        let mut c = client.inner.lock();
        c.state = SocketState::Connected;
        c.pair = Some(pair);
        c.is_side_a = false;
        c.peer_addr = Some(UnixAddr::Path(path.to_string()));
    }

    Ok(client)
}

/// Accept a pending connection from a listening socket.
pub fn unix_accept(socket: &Arc<UnixSocket>) -> KernelResult<Arc<UnixSocket>> {
    let mut inner = socket.inner.lock();
    if inner.state != SocketState::Listening {
        return Err(KernelError::InvalidInput);
    }
    inner.accept_queue.pop_front().ok_or(KernelError::Busy)
}

/// Unbind and remove from registry.
pub fn unix_unbind(path: &str) {
    UNIX_REGISTRY.lock().remove(path);
}

pub(super) fn registered_names() -> usize {
    UNIX_REGISTRY.lock().len()
}
