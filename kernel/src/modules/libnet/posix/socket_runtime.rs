#[cfg(feature = "network_transport")]
use super::*;
#[cfg(feature = "network_transport")]
use crate::modules::vfs::types::{FileStats, SeekFrom};
#[cfg(feature = "network_transport")]
use alloc::collections::VecDeque;
#[cfg(feature = "network_transport")]
use alloc::sync::Arc;
#[cfg(feature = "network_transport")]
use alloc::vec::Vec;
#[cfg(feature = "network_transport")]
use core::any::Any;
#[cfg(feature = "network_transport")]
use core::sync::atomic::{AtomicU16, Ordering};
#[cfg(feature = "network_transport")]
use spin::Mutex;

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone)]
pub(super) struct DatagramState {
    pub(super) socket: Option<crate::modules::libnet::LibUdpSocket>,
    pub(super) local_port: Option<u16>,
    pub(super) peer_port: Option<u16>,
    pub(super) pending: VecDeque<crate::modules::libnet::UdpDatagram>,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone)]
pub(super) enum StreamState {
    Unbound {
        local_port: Option<u16>,
    },
    Listening {
        listener: crate::modules::libnet::LibTcpListener,
        pending_accepts: VecDeque<crate::modules::libnet::LibTcpStream>,
    },
    Connected {
        stream: crate::modules::libnet::LibTcpStream,
        pending: VecDeque<Vec<u8>>,
    },
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone)]
pub(super) enum PosixSocket {
    Datagram(DatagramState),
    Stream(StreamState),
}

#[cfg(feature = "network_transport")]
pub struct SocketFile {
    pub(super) inner: Arc<Mutex<PosixSocket>>,
    pub(super) flags: Arc<Mutex<SocketFlags>>,
}

#[cfg(feature = "network_transport")]
impl File for SocketFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut inner = self.inner.lock();
        let flags = self.flags.lock();

        match &mut *inner {
            PosixSocket::Stream(StreamState::Connected { stream, pending }) => {
                if let Some(chunk) = pending.pop_front() {
                    let n = chunk.len().min(buf.len());
                    buf[..n].copy_from_slice(&chunk[..n]);
                    if n < chunk.len() {
                        pending.push_front(chunk[n..].to_vec());
                    }
                    Ok(n)
                } else if let Some(chunk) = stream.recv() {
                    let n = chunk.len().min(buf.len());
                    buf[..n].copy_from_slice(&chunk[..n]);
                    if n < chunk.len() {
                        pending.push_back(chunk[n..].to_vec());
                    }
                    Ok(n)
                } else if flags.nonblocking {
                    Err("would block")
                } else {
                    Err("timeout")
                }
            }
            PosixSocket::Datagram(state) => {
                if let Some(datagram) = state.pending.pop_front() {
                    let n = datagram.payload.len().min(buf.len());
                    buf[..n].copy_from_slice(&datagram.payload[..n]);
                    Ok(n)
                } else {
                    ensure_datagram_socket(state)?;
                    if let Some(datagram) = state.socket.as_ref().unwrap().recv() {
                        let n = datagram.payload.len().min(buf.len());
                        buf[..n].copy_from_slice(&datagram.payload[..n]);
                        Ok(n)
                    } else if flags.nonblocking {
                        Err("would block")
                    } else {
                        Err("timeout")
                    }
                }
            }
            _ => Err("not connected"),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let mut inner = self.inner.lock();
        let flags = self.flags.lock();
        if flags.shutdown_write {
            return Err("shutdown");
        }

        match &mut *inner {
            PosixSocket::Stream(StreamState::Connected { stream, .. }) => {
                stream.send(buf).map_err(|_| "send failed")?;
                Ok(buf.len())
            }
            PosixSocket::Datagram(state) => {
                ensure_datagram_socket(state)?;
                let dst = state.peer_port.ok_or("not connected")?;
                state
                    .socket
                    .as_ref()
                    .unwrap()
                    .send_to(dst, buf)
                    .map_err(|_| "send failed")?;
                Ok(buf.len())
            }
            _ => Err("not connected"),
        }
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Err("illegal seek on socket")
    }

    fn poll_events(&self) -> crate::modules::vfs::types::PollEvents {
        let mut revents = 0;
        let mut inner = self.inner.lock();
        let flags = self.flags.lock();

        match &mut *inner {
            PosixSocket::Datagram(state) => {
                if !state.pending.is_empty() {
                    revents |= crate::modules::posix_consts::net::POLLIN as u32;
                } else if let Ok(()) = ensure_datagram_socket(state) {
                    if let Some(d) = state.socket.as_ref().unwrap().recv() {
                        state.pending.push_back(d);
                        revents |= crate::modules::posix_consts::net::POLLIN as u32;
                    }
                }
                revents |= crate::modules::posix_consts::net::POLLOUT as u32;
            }
            PosixSocket::Stream(s) => match s {
                StreamState::Listening {
                    listener,
                    pending_accepts,
                } => {
                    if !pending_accepts.is_empty() {
                        revents |= crate::modules::posix_consts::net::POLLIN as u32;
                    } else if let Some(stream) = listener.accept() {
                        pending_accepts.push_back(stream);
                        revents |= crate::modules::posix_consts::net::POLLIN as u32;
                    }
                }
                StreamState::Connected { stream, pending } => {
                    if !pending.is_empty() {
                        revents |= crate::modules::posix_consts::net::POLLIN as u32;
                    } else if let Some(chunk) = stream.recv() {
                        pending.push_back(chunk);
                        revents |= crate::modules::posix_consts::net::POLLIN as u32;
                    }
                    revents |= crate::modules::posix_consts::net::POLLOUT as u32;
                }
                _ => {}
            },
        }

        if flags.shutdown_read {
            revents |= 0x10;
        }
        if flags.shutdown_write {
            revents |= 0x08;
        }

        crate::modules::vfs::types::PollEvents::from_bits_truncate(revents)
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o140666,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(feature = "network_transport")]
static NEXT_EPHEMERAL_PORT: AtomicU16 = AtomicU16::new(40_000);

#[cfg(feature = "network_transport")]
pub(super) fn alloc_ephemeral_port() -> u16 {
    let ephemeral_start = crate::config::KernelConfig::libnet_posix_ephemeral_start();
    loop {
        let current = NEXT_EPHEMERAL_PORT.load(Ordering::Relaxed);
        if current >= ephemeral_start {
            break;
        }
        if NEXT_EPHEMERAL_PORT
            .compare_exchange_weak(
                current,
                ephemeral_start,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            break;
        }
    }

    let mut port = NEXT_EPHEMERAL_PORT.fetch_add(1, Ordering::Relaxed);
    if port < ephemeral_start {
        NEXT_EPHEMERAL_PORT.store(ephemeral_start.saturating_add(1), Ordering::Relaxed);
        port = ephemeral_start;
    }
    if port == u16::MAX {
        NEXT_EPHEMERAL_PORT.store(ephemeral_start.saturating_add(1), Ordering::Relaxed);
    }
    port
}

#[cfg(feature = "network_transport")]
pub(super) fn with_socket<F, R>(fd: u32, f: F) -> Result<R, &'static str>
where
    F: FnOnce(&SocketFile) -> Result<R, &'static str>,
{
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or("invalid socket fd")?;
    let handle = desc.file.handle.lock();
    if let Some(socket_file) = handle.as_any().downcast_ref::<SocketFile>() {
        f(socket_file)
    } else {
        Err("not a socket")
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn with_socket_mut<F, R>(fd: u32, f: F) -> Result<R, &'static str>
where
    F: FnOnce(&mut SocketFile) -> Result<R, &'static str>,
{
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or("invalid socket fd")?;
    let mut handle = desc.file.handle.lock();
    if let Some(socket_file) = handle.as_any_mut().downcast_mut::<SocketFile>() {
        f(socket_file)
    } else {
        Err("not a socket")
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn socket_flags(fd: u32) -> Result<SocketFlags, &'static str> {
    with_socket(fd, |s| Ok(*s.flags.lock()))
}

#[cfg(feature = "network_transport")]
pub(super) fn set_socket_flags(
    fd: u32,
    apply: impl FnOnce(&mut SocketFlags),
) -> Result<(), &'static str> {
    with_socket(fd, |s| {
        let mut flags = s.flags.lock();
        apply(&mut flags);
        Ok(())
    })
}

#[cfg(feature = "network_transport")]
pub(super) fn ensure_datagram_socket(state: &mut DatagramState) -> Result<(), &'static str> {
    if state.socket.is_some() {
        return Ok(());
    }
    let local_port = state.local_port.unwrap_or_else(alloc_ephemeral_port);
    let socket = crate::modules::libnet::udp_bind(local_port)?;
    state.local_port = Some(local_port);
    state.socket = Some(socket);
    Ok(())
}

#[cfg(feature = "network_transport")]
pub(super) fn set_last_error(fd: u32, error: PosixErrno) {
    let mut table = crate::modules::posix::fs::FILE_TABLE.lock();
    if let Some(desc) = table.get_mut(&fd) {
        let mut handle = desc.file.handle.lock();
        if let Some(socket_file) = handle.as_any_mut().downcast_mut::<SocketFile>() {
            socket_file.flags.lock().last_error = Some(error);
        }
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn clear_last_error(fd: u32) {
    let mut table = crate::modules::posix::fs::FILE_TABLE.lock();
    if let Some(desc) = table.get_mut(&fd) {
        let mut handle = desc.file.handle.lock();
        if let Some(socket_file) = handle.as_any_mut().downcast_mut::<SocketFile>() {
            socket_file.flags.lock().last_error = None;
        }
    }
}

#[cfg(feature = "network_transport")]
pub(super) fn poll_transport_hint() {
    let _ = crate::modules::libnet::l34::poll_transport_once();
}

#[cfg(feature = "network_transport")]
pub(super) fn ensure_posix_available() -> Result<(), &'static str> {
    crate::modules::libnet::policy::ensure_l34_enabled()
}
