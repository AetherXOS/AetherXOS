use super::*;
use alloc::collections::VecDeque;

#[cfg(feature = "network_transport")]
pub fn tcp_listen(port: u16) -> Result<TcpListener, &'static str> {
    if port == 0 {
        return Err("invalid tcp port");
    }
    TCP_LISTEN_CALLS.fetch_add(1, Ordering::Relaxed);
    TCP_LISTENERS.lock().insert(port, ());
    TCP_PENDING_ACCEPT
        .lock()
        .entry(port)
        .or_insert_with(VecDeque::new);
    TCP_STREAM_QUEUES
        .lock()
        .entry(port)
        .or_insert_with(VecDeque::new);
    Ok(TcpListener { local_port: port })
}

#[cfg(feature = "network_transport")]
pub fn tcp_connect(local_port: u16, remote_port: u16) -> Result<TcpStream, &'static str> {
    if local_port == 0 || remote_port == 0 {
        return Err("invalid tcp port");
    }
    TCP_CONNECT_CALLS.fetch_add(1, Ordering::Relaxed);
    if !TCP_LISTENERS.lock().contains_key(&remote_port) {
        return Err("connection refused");
    }
    let stream = TcpStream {
        local_port,
        peer_port: remote_port,
    };
    TCP_PENDING_ACCEPT
        .lock()
        .entry(remote_port)
        .or_insert_with(VecDeque::new)
        .push_back(local_port);
    Ok(stream)
}