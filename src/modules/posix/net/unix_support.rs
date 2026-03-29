use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU16, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use super::{map_net_errno, PosixErrno, PosixMsgFlags, PosixSocketAddrV4};

const UNIX_ENDPOINT_BASE_PORT: u16 = 52_000;
const UNIX_ENDPOINT_ALLOC_ATTEMPTS: usize = 2048;

static NEXT_UNIX_ENDPOINT_PORT: AtomicU16 = AtomicU16::new(UNIX_ENDPOINT_BASE_PORT);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SockAddrUn {
    bytes: [u8; crate::modules::posix_consts::net::UNIX_PATH_MAX],
    len: usize,
}

impl SockAddrUn {
    pub const FAMILY_RAW: u16 = crate::modules::posix_consts::net::AF_UNIX as u16;

    pub fn from_path(path: &[u8]) -> Result<Self, PosixErrno> {
        if path.is_empty() || path.len() > crate::modules::posix_consts::net::UNIX_PATH_MAX {
            return Err(PosixErrno::Invalid);
        }
        if path[0] != 0 && path.iter().any(|&b| b == 0) {
            return Err(PosixErrno::Invalid);
        }
        if path[0] == 0 && path.len() <= 1 {
            return Err(PosixErrno::Invalid);
        }

        let mut bytes = [0u8; crate::modules::posix_consts::net::UNIX_PATH_MAX];
        bytes[..path.len()].copy_from_slice(path);
        Ok(Self {
            bytes,
            len: path.len(),
        })
    }

    pub fn as_path_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_abstract(&self) -> bool {
        self.len > 0 && self.bytes[0] == 0
    }

    pub fn encode_raw(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + self.len);
        out.extend_from_slice(&Self::FAMILY_RAW.to_ne_bytes());
        out.extend_from_slice(self.as_path_bytes());
        out
    }

    pub fn decode_raw(raw: &[u8]) -> Result<Self, PosixErrno> {
        if raw.len() < 2 {
            return Err(PosixErrno::Invalid);
        }
        let family = u16::from_ne_bytes([raw[0], raw[1]]);
        if family != Self::FAMILY_RAW {
            return Err(PosixErrno::Invalid);
        }
        Self::from_path(&raw[2..])
    }
}

lazy_static! {
    static ref UNIX_ENDPOINTS: Mutex<BTreeMap<Vec<u8>, u16>> = Mutex::new(BTreeMap::new());
    static ref UNIX_BOUND_BY_FD: Mutex<BTreeMap<u32, Vec<u8>>> = Mutex::new(BTreeMap::new());
}

fn normalize_unix_addr_key(path: &[u8]) -> Result<Vec<u8>, PosixErrno> {
    if path.is_empty() || path.len() > crate::modules::posix_consts::net::UNIX_PATH_MAX {
        return Err(PosixErrno::Invalid);
    }
    if path[0] == 0 {
        if path.len() <= 1 {
            return Err(PosixErrno::Invalid);
        }
        return Ok(path.to_vec());
    }
    if path.iter().any(|&b| b == 0) {
        return Err(PosixErrno::Invalid);
    }
    Ok(path.to_vec())
}

fn alloc_unix_endpoint_port() -> Result<u16, PosixErrno> {
    for _ in 0..UNIX_ENDPOINT_ALLOC_ATTEMPTS {
        let port = NEXT_UNIX_ENDPOINT_PORT.fetch_add(1, Ordering::Relaxed);
        if port < UNIX_ENDPOINT_BASE_PORT {
            NEXT_UNIX_ENDPOINT_PORT
                .store(UNIX_ENDPOINT_BASE_PORT.saturating_add(1), Ordering::Relaxed);
            continue;
        }
        let used = UNIX_ENDPOINTS.lock().values().copied().any(|v| v == port);
        if !used {
            return Ok(port);
        }
    }
    Err(PosixErrno::AddrInUse)
}

fn unix_port_by_path(path: &[u8]) -> Result<u16, PosixErrno> {
    let key = normalize_unix_addr_key(path)?;
    UNIX_ENDPOINTS
        .lock()
        .get(&key)
        .copied()
        .ok_or(PosixErrno::NoEntry)
}

fn unix_path_by_port(port: u16) -> Option<Vec<u8>> {
    let endpoints = UNIX_ENDPOINTS.lock();
    endpoints
        .iter()
        .find_map(|(path, p)| if *p == port { Some(path.clone()) } else { None })
}

pub fn unix_bind_path(fd: u32, path: &[u8]) -> Result<(), PosixErrno> {
    unix_bind_addr(fd, path)
}

pub fn unix_bind_addr(fd: u32, path: &[u8]) -> Result<(), PosixErrno> {
    let key = normalize_unix_addr_key(path)?;
    {
        let bound = UNIX_BOUND_BY_FD.lock();
        if bound.contains_key(&fd) {
            return Err(PosixErrno::AddrInUse);
        }
    }

    {
        let endpoints = UNIX_ENDPOINTS.lock();
        if endpoints.contains_key(&key) {
            return Err(PosixErrno::AddrInUse);
        }
    }

    let port = alloc_unix_endpoint_port()?;
    crate::modules::libnet::posix_bind_errno(fd, PosixSocketAddrV4::localhost(port))
        .map_err(map_net_errno)?;

    UNIX_ENDPOINTS.lock().insert(key.clone(), port);
    UNIX_BOUND_BY_FD.lock().insert(fd, key);
    Ok(())
}

#[inline(always)]
pub fn unix_bind_sockaddr(fd: u32, addr: &SockAddrUn) -> Result<(), PosixErrno> {
    unix_bind_addr(fd, addr.as_path_bytes())
}

pub fn unix_connect_path(fd: u32, path: &[u8]) -> Result<(), PosixErrno> {
    unix_connect_addr(fd, path)
}

pub fn unix_connect_addr(fd: u32, path: &[u8]) -> Result<(), PosixErrno> {
    let port = unix_port_by_path(path)?;
    crate::modules::libnet::posix_connect_errno(fd, PosixSocketAddrV4::localhost(port))
        .map_err(map_net_errno)
}

#[inline(always)]
pub fn unix_connect_sockaddr(fd: u32, addr: &SockAddrUn) -> Result<(), PosixErrno> {
    unix_connect_addr(fd, addr.as_path_bytes())
}

pub fn unix_listen(fd: u32, backlog: usize) -> Result<(), PosixErrno> {
    crate::modules::libnet::posix_listen_errno(fd, backlog).map_err(map_net_errno)
}

pub fn unix_accept(fd: u32) -> Result<u32, PosixErrno> {
    crate::modules::libnet::posix_accept_errno(fd).map_err(map_net_errno)
}

pub fn unix_unlink_path(path: &[u8]) -> Result<(), PosixErrno> {
    unix_unlink_addr(path)
}

pub fn unix_unlink_addr(path: &[u8]) -> Result<(), PosixErrno> {
    let key = normalize_unix_addr_key(path)?;
    let removed = UNIX_ENDPOINTS.lock().remove(&key);
    if removed.is_none() {
        return Err(PosixErrno::NoEntry);
    }
    let mut bound = UNIX_BOUND_BY_FD.lock();
    let remove_fd = bound.iter().find_map(|(fd, p)| {
        if p.as_slice() == key.as_slice() {
            Some(*fd)
        } else {
            None
        }
    });
    if let Some(fd) = remove_fd {
        bound.remove(&fd);
    }
    Ok(())
}

#[inline(always)]
pub fn unix_unlink_sockaddr(addr: &SockAddrUn) -> Result<(), PosixErrno> {
    unix_unlink_addr(addr.as_path_bytes())
}

pub fn unix_sendto_path(fd: u32, path: &[u8], payload: &[u8]) -> Result<usize, PosixErrno> {
    let port = unix_port_by_path(path)?;
    crate::modules::libnet::posix_sendto_errno(fd, PosixSocketAddrV4::localhost(port), payload)
        .map_err(map_net_errno)
}

pub fn unix_sendto_sockaddr(
    fd: u32,
    addr: &SockAddrUn,
    payload: &[u8],
) -> Result<usize, PosixErrno> {
    unix_sendto_path(fd, addr.as_path_bytes(), payload)
}

pub fn unix_recvfrom_addr(
    fd: u32,
    flags: PosixMsgFlags,
) -> Result<(SockAddrUn, Vec<u8>), PosixErrno> {
    let packet = super::recvfrom_flags(fd, flags)?;
    let Some(path) = unix_path_by_port(packet.addr.port) else {
        return Err(PosixErrno::NoEntry);
    };
    let addr = SockAddrUn::from_path(&path)?;
    Ok((addr, packet.payload))
}

pub fn on_close_fd(fd: u32) {
    if let Some(path) = UNIX_BOUND_BY_FD.lock().remove(&fd) {
        UNIX_ENDPOINTS.lock().remove(&path);
    }
}
