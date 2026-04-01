/// Socket Options — setsockopt / getsockopt framework.
///
/// Provides a configurable, extensible socket option layer for TCP/UDP sockets.
/// Each option is identified by level + optname and can be get/set on a per-socket
/// basis.
///
/// Supported options:
/// - SOL_SOCKET: SO_REUSEADDR, SO_REUSEPORT, SO_KEEPALIVE, SO_LINGER,
///               SO_RCVBUF, SO_SNDBUF, SO_BROADCAST, SO_RCVTIMEO, SO_SNDTIMEO
/// - IPPROTO_TCP: TCP_NODELAY, TCP_KEEPIDLE, TCP_KEEPINTVL, TCP_KEEPCNT
/// - IPPROTO_IP: IP_TTL, IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP, IP_MULTICAST_TTL
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ─── Telemetry ───────────────────────────────────────────────────────

static SETSOCKOPT_CALLS: AtomicU64 = AtomicU64::new(0);
static GETSOCKOPT_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct SockOptStats {
    pub setsockopt_calls: u64,
    pub getsockopt_calls: u64,
}

pub fn sockopt_stats() -> SockOptStats {
    SockOptStats {
        setsockopt_calls: SETSOCKOPT_CALLS.load(Ordering::Relaxed),
        getsockopt_calls: GETSOCKOPT_CALLS.load(Ordering::Relaxed),
    }
}

// ─── Option Level Constants ──────────────────────────────────────────

pub const SOL_SOCKET: i32 = 1;
pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;
pub const IPPROTO_IP: i32 = 0;
pub const IPPROTO_IPV6: i32 = 41;

// ─── Socket Option Constants ────────────────────────────────────────

// SOL_SOCKET options
pub const SO_REUSEADDR: i32 = 2;
pub const SO_REUSEPORT: i32 = 15;
pub const SO_KEEPALIVE: i32 = 9;
pub const SO_LINGER: i32 = 13;
pub const SO_RCVBUF: i32 = 8;
pub const SO_SNDBUF: i32 = 7;
pub const SO_BROADCAST: i32 = 6;
pub const SO_RCVTIMEO: i32 = 20;
pub const SO_SNDTIMEO: i32 = 21;
pub const SO_ERROR: i32 = 4;
pub const SO_TYPE: i32 = 3;
pub const SO_NONBLOCK: i32 = 100; // Our custom extension

// TCP options
pub const TCP_NODELAY: i32 = 1;
pub const TCP_KEEPIDLE: i32 = 4;
pub const TCP_KEEPINTVL: i32 = 5;
pub const TCP_KEEPCNT: i32 = 6;

// IP options
pub const IP_TTL: i32 = 2;
pub const IP_ADD_MEMBERSHIP: i32 = 35;
pub const IP_DROP_MEMBERSHIP: i32 = 36;
pub const IP_MULTICAST_TTL: i32 = 33;
pub const IP_MULTICAST_IF: i32 = 32;
pub const IP_MULTICAST_LOOP: i32 = 34;

// IPV6 options
pub const IPV6_V6ONLY: i32 = 26;

// ─── Linger ──────────────────────────────────────────────────────────

/// SO_LINGER option value.
#[derive(Debug, Clone, Copy)]
pub struct Linger {
    /// Enable linger on close.
    pub l_onoff: bool,
    /// Linger timeout in seconds.
    pub l_linger: u32,
}

impl Default for Linger {
    fn default() -> Self {
        Self {
            l_onoff: false,
            l_linger: 0,
        }
    }
}

// ─── Multicast Group ─────────────────────────────────────────────────

/// Multicast group membership (for IP_ADD_MEMBERSHIP / IP_DROP_MEMBERSHIP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub struct MulticastGroup {
    /// Multicast group address (IPv4 as u32, big-endian).
    pub group_addr: u32,
    /// Local interface address (0 = any).
    pub local_addr: u32,
}

// ─── Per-Socket Options ──────────────────────────────────────────────

/// Socket option state for a single socket.
#[derive(Debug, Clone)]
pub struct SocketOptions {
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub keep_alive: bool,
    pub linger: Linger,
    pub recv_buf_size: u32,
    pub send_buf_size: u32,
    pub broadcast: bool,
    pub nonblocking: bool,
    /// Receive timeout in milliseconds (0 = no timeout).
    pub recv_timeout_ms: u64,
    /// Send timeout in milliseconds (0 = no timeout).
    pub send_timeout_ms: u64,
    /// TCP: disable Nagle's algorithm.
    pub tcp_nodelay: bool,
    /// TCP: keep-alive idle time (seconds).
    pub tcp_keepidle: u32,
    /// TCP: keep-alive probe interval (seconds).
    pub tcp_keepintvl: u32,
    /// TCP: keep-alive probe count.
    pub tcp_keepcnt: u32,
    /// IP: Time To Live.
    pub ip_ttl: u8,
    /// IP: Multicast TTL.
    pub ip_multicast_ttl: u8,
    /// IP: Multicast loopback.
    pub ip_multicast_loop: bool,
    /// IPv6: Only accept IPv6 connections.
    pub ipv6_v6only: bool,
    /// Active multicast group memberships.
    pub multicast_groups: Vec<MulticastGroup>,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            keep_alive: false,
            linger: Linger::default(),
            recv_buf_size: 65536,
            send_buf_size: 65536,
            broadcast: false,
            nonblocking: false,
            recv_timeout_ms: 0,
            send_timeout_ms: 0,
            tcp_nodelay: false,
            tcp_keepidle: 7200,
            tcp_keepintvl: 75,
            tcp_keepcnt: 9,
            ip_ttl: 64,
            ip_multicast_ttl: 1,
            ip_multicast_loop: true,
            ipv6_v6only: false,
            multicast_groups: Vec::new(),
        }
    }
}

// ─── Setsockopt / Getsockopt ─────────────────────────────────────────

/// Error from socket option operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockOptError {
    InvalidLevel,
    InvalidOption,
    InvalidValue,
    NotSupported,
}

impl SocketOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a socket option. `value` is the raw bytes of the option value.
    pub fn setsockopt(
        &mut self,
        level: i32,
        optname: i32,
        value: &[u8],
    ) -> Result<(), SockOptError> {
        SETSOCKOPT_CALLS.fetch_add(1, Ordering::Relaxed);

        // Helper to read a 4-byte int from value.
        let read_i32 = || -> Result<i32, SockOptError> {
            if value.len() < 4 {
                return Err(SockOptError::InvalidValue);
            }
            Ok(i32::from_ne_bytes([value[0], value[1], value[2], value[3]]))
        };

        let read_u32 = || -> Result<u32, SockOptError> { read_i32().map(|v| v as u32) };

        match level {
            SOL_SOCKET => match optname {
                SO_REUSEADDR => {
                    self.reuse_addr = read_i32()? != 0;
                    Ok(())
                }
                SO_REUSEPORT => {
                    self.reuse_port = read_i32()? != 0;
                    Ok(())
                }
                SO_KEEPALIVE => {
                    self.keep_alive = read_i32()? != 0;
                    Ok(())
                }
                SO_BROADCAST => {
                    self.broadcast = read_i32()? != 0;
                    Ok(())
                }
                SO_NONBLOCK => {
                    self.nonblocking = read_i32()? != 0;
                    Ok(())
                }
                SO_RCVBUF => {
                    self.recv_buf_size = read_u32()?;
                    Ok(())
                }
                SO_SNDBUF => {
                    self.send_buf_size = read_u32()?;
                    Ok(())
                }
                SO_RCVTIMEO => {
                    self.recv_timeout_ms = read_u32()? as u64;
                    Ok(())
                }
                SO_SNDTIMEO => {
                    self.send_timeout_ms = read_u32()? as u64;
                    Ok(())
                }
                SO_LINGER => {
                    if value.len() < 8 {
                        return Err(SockOptError::InvalidValue);
                    }
                    let onoff = i32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    let linger = i32::from_ne_bytes([value[4], value[5], value[6], value[7]]);
                    self.linger = Linger {
                        l_onoff: onoff != 0,
                        l_linger: linger as u32,
                    };
                    Ok(())
                }
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_TCP => match optname {
                TCP_NODELAY => {
                    self.tcp_nodelay = read_i32()? != 0;
                    Ok(())
                }
                TCP_KEEPIDLE => {
                    self.tcp_keepidle = read_u32()?;
                    Ok(())
                }
                TCP_KEEPINTVL => {
                    self.tcp_keepintvl = read_u32()?;
                    Ok(())
                }
                TCP_KEEPCNT => {
                    self.tcp_keepcnt = read_u32()?;
                    Ok(())
                }
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_IP => match optname {
                IP_TTL => {
                    self.ip_ttl = read_i32()? as u8;
                    Ok(())
                }
                IP_MULTICAST_TTL => {
                    self.ip_multicast_ttl = read_i32()? as u8;
                    Ok(())
                }
                IP_MULTICAST_LOOP => {
                    self.ip_multicast_loop = read_i32()? != 0;
                    Ok(())
                }
                IP_ADD_MEMBERSHIP => {
                    if value.len() < 8 {
                        return Err(SockOptError::InvalidValue);
                    }
                    let group = u32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    let local = u32::from_ne_bytes([value[4], value[5], value[6], value[7]]);
                    let mg = MulticastGroup {
                        group_addr: group,
                        local_addr: local,
                    };
                    if !self.multicast_groups.contains(&mg) {
                        self.multicast_groups.push(mg);
                    }
                    Ok(())
                }
                IP_DROP_MEMBERSHIP => {
                    if value.len() < 8 {
                        return Err(SockOptError::InvalidValue);
                    }
                    let group = u32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    let local = u32::from_ne_bytes([value[4], value[5], value[6], value[7]]);
                    let mg = MulticastGroup {
                        group_addr: group,
                        local_addr: local,
                    };
                    self.multicast_groups.retain(|g| g != &mg);
                    Ok(())
                }
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_IPV6 => match optname {
                IPV6_V6ONLY => {
                    self.ipv6_v6only = read_i32()? != 0;
                    Ok(())
                }
                _ => Err(SockOptError::InvalidOption),
            },
            _ => Err(SockOptError::InvalidLevel),
        }
    }

    /// Get a socket option value as i32 (for most options).
    pub fn getsockopt(&self, level: i32, optname: i32) -> Result<i64, SockOptError> {
        GETSOCKOPT_CALLS.fetch_add(1, Ordering::Relaxed);
        match level {
            SOL_SOCKET => match optname {
                SO_REUSEADDR => Ok(self.reuse_addr as i64),
                SO_REUSEPORT => Ok(self.reuse_port as i64),
                SO_KEEPALIVE => Ok(self.keep_alive as i64),
                SO_BROADCAST => Ok(self.broadcast as i64),
                SO_NONBLOCK => Ok(self.nonblocking as i64),
                SO_RCVBUF => Ok(self.recv_buf_size as i64),
                SO_SNDBUF => Ok(self.send_buf_size as i64),
                SO_RCVTIMEO => Ok(self.recv_timeout_ms as i64),
                SO_SNDTIMEO => Ok(self.send_timeout_ms as i64),
                SO_TYPE => Ok(0),  // Caller should set based on socket type.
                SO_ERROR => Ok(0), // Pending error.
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_TCP => match optname {
                TCP_NODELAY => Ok(self.tcp_nodelay as i64),
                TCP_KEEPIDLE => Ok(self.tcp_keepidle as i64),
                TCP_KEEPINTVL => Ok(self.tcp_keepintvl as i64),
                TCP_KEEPCNT => Ok(self.tcp_keepcnt as i64),
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_IP => match optname {
                IP_TTL => Ok(self.ip_ttl as i64),
                IP_MULTICAST_TTL => Ok(self.ip_multicast_ttl as i64),
                IP_MULTICAST_LOOP => Ok(self.ip_multicast_loop as i64),
                _ => Err(SockOptError::InvalidOption),
            },
            IPPROTO_IPV6 => match optname {
                IPV6_V6ONLY => Ok(self.ipv6_v6only as i64),
                _ => Err(SockOptError::InvalidOption),
            },
            _ => Err(SockOptError::InvalidLevel),
        }
    }
}

// ─── Socket Option Registry ─────────────────────────────────────────

/// Per-socket-fd option storage.
pub struct SocketOptionRegistry {
    sockets: BTreeMap<i32, SocketOptions>,
}

impl SocketOptionRegistry {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
        }
    }

    /// Register a new socket with default options.
    pub fn register(&mut self, fd: i32) {
        self.sockets.insert(fd, SocketOptions::default());
    }

    /// Unregister a socket (on close).
    pub fn unregister(&mut self, fd: i32) {
        self.sockets.remove(&fd);
    }

    /// Get options for a socket.
    pub fn get(&self, fd: i32) -> Option<&SocketOptions> {
        self.sockets.get(&fd)
    }

    /// Get mutable options for a socket.
    pub fn get_mut(&mut self, fd: i32) -> Option<&mut SocketOptions> {
        self.sockets.get_mut(&fd)
    }
}
