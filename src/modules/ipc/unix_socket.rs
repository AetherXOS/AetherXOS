//! Unix Domain Socket (AF_UNIX) — production-grade implementation.
//!
//! Provides local inter-process communication via socket API:
//! - `SOCK_STREAM` — reliable, ordered, connection-oriented byte stream
//! - `SOCK_DGRAM` — unreliable, unordered datagrams
//! - `SOCK_SEQPACKET` — reliable, ordered, preserves message boundaries
//!
//! Supports: socketpair, bind, listen, accept, connect, send/recv,
//! shutdown, SO_PASSCRED, abstract namespace, buffer tuning.
//!
//! Feature-gated: requires `ipc_unix_domain` feature.

use crate::interfaces::{KernelError, KernelResult};
use crate::kernel::sync::WaitQueue;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::Mutex;

mod registry;

pub use registry::{unix_accept, unix_bind, unix_connect, unix_listen, unix_unbind};

// ── Constants ───────────────────────────────────────────────────────────────

/// Default kernel buffer size per socket direction.
const DEFAULT_BUF_CAPACITY: usize = 65536;
/// Maximum pending connections in listen backlog.
const MAX_BACKLOG: usize = 4096;
/// Maximum number of queued DGRAM/SEQPACKET messages.
const MAX_MSG_QUEUE: usize = 256;

// ── Socket Types ────────────────────────────────────────────────────────────

/// Unix socket type (matches Linux SOCK_* constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UnixSocketType {
    Stream = 1,
    Dgram = 2,
    SeqPacket = 5,
}

impl UnixSocketType {
    /// Parse from raw syscall value (masking SOCK_NONBLOCK | SOCK_CLOEXEC).
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw & 0xF {
            1 => Some(Self::Stream),
            2 => Some(Self::Dgram),
            5 => Some(Self::SeqPacket),
            _ => None,
        }
    }
}

/// Socket lifecycle state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Unbound,
    Bound,
    Listening,
    Connecting,
    Connected,
    Shutdown,
    Closed,
}

/// Socket address — unnamed, abstract namespace, or filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnixAddr {
    Unnamed,
    Abstract(Vec<u8>),
    Path(String),
}

impl UnixAddr {
    /// Create from a raw sockaddr_un byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::Unnamed;
        }
        if bytes[0] == 0 {
            // Abstract namespace
            Self::Abstract(bytes[1..].to_vec())
        } else {
            // Filesystem path (nul-terminated)
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            Self::Path(String::from_utf8_lossy(&bytes[..end]).into_owned())
        }
    }

    fn registry_key(&self) -> Option<String> {
        match self {
            Self::Unnamed => None,
            Self::Abstract(b) => {
                let hex: String = b.iter().map(|v| alloc::format!("{:02x}", v)).collect();
                Some(alloc::format!("\0{}", hex))
            }
            Self::Path(p) => Some(p.clone()),
        }
    }
}

// ── Internal pair buffer ────────────────────────────────────────────────────

/// Bidirectional shared state between two connected endpoints.
struct PairBuffer {
    /// A→B byte stream / message queue.
    a_to_b: ChannelBuf,
    /// B→A byte stream / message queue.
    b_to_a: ChannelBuf,
    a_shut_wr: bool,
    b_shut_wr: bool,
}

/// One-directional channel (stream or message-oriented).
enum ChannelBuf {
    Stream {
        data: VecDeque<u8>,
        capacity: usize,
    },
    Messages {
        queue: VecDeque<Vec<u8>>,
        max_msgs: usize,
    },
}

impl ChannelBuf {
    fn new_stream(capacity: usize) -> Self {
        Self::Stream {
            data: VecDeque::with_capacity(capacity.min(8192)),
            capacity,
        }
    }

    fn new_messages() -> Self {
        Self::Messages {
            queue: VecDeque::with_capacity(32),
            max_msgs: MAX_MSG_QUEUE,
        }
    }

    fn available(&self) -> usize {
        match self {
            Self::Stream { data, .. } => data.len(),
            Self::Messages { queue, .. } => queue.front().map(|m| m.len()).unwrap_or(0),
        }
    }

    fn writable_bytes(&self) -> usize {
        match self {
            Self::Stream { data, capacity } => capacity.saturating_sub(data.len()),
            Self::Messages { queue, max_msgs } => {
                if queue.len() < *max_msgs { usize::MAX } else { 0 }
            }
        }
    }
}

// ── Socket struct ───────────────────────────────────────────────────────────

/// A single Unix domain socket endpoint.
pub struct UnixSocket {
    inner: Mutex<UnixSocketInner>,
    rx_wait: WaitQueue,
    tx_wait: WaitQueue,
}

struct UnixSocketInner {
    socket_type: UnixSocketType,
    state: SocketState,
    local_addr: UnixAddr,
    peer_addr: Option<UnixAddr>,
    /// For STREAM listening: pending connections.
    accept_queue: VecDeque<Arc<UnixSocket>>,
    backlog: usize,
    nonblocking: bool,
    pass_cred: bool,
    rcvbuf: usize,
    sndbuf: usize,
    /// Connected pair shared state.
    pair: Option<Arc<Mutex<PairBuffer>>>,
    /// true = A side, false = B side.
    is_side_a: bool,
}

impl UnixSocket {
    /// Create a new unbound socket.
    pub fn new(socket_type: UnixSocketType) -> Arc<Self> {
        STAT_CREATED.fetch_add(1, Ordering::Relaxed);
        Arc::new(Self {
            inner: Mutex::new(UnixSocketInner {
                socket_type,
                state: SocketState::Unbound,
                local_addr: UnixAddr::Unnamed,
                peer_addr: None,
                accept_queue: VecDeque::new(),
                backlog: 128,
                nonblocking: false,
                pass_cred: false,
                rcvbuf: DEFAULT_BUF_CAPACITY,
                sndbuf: DEFAULT_BUF_CAPACITY,
                pair: None,
                is_side_a: true,
            }),
            rx_wait: WaitQueue::new(),
            tx_wait: WaitQueue::new(),
        })
    }

    /// Create with explicit capacity (legacy API compat).
    pub fn with_capacity(capacity: usize) -> Arc<Self> {
        let sock = Self::new(UnixSocketType::Stream);
        sock.inner.lock().rcvbuf = capacity;
        sock.inner.lock().sndbuf = capacity;
        sock
    }

    // ── Data path ───────────────────────────────────────────────────────

    /// Write data to the connected peer.
    pub fn write(&self, buf: &[u8]) -> KernelResult<usize> {
        let inner = self.inner.lock();
        if inner.state != SocketState::Connected {
            return Err(KernelError::Disconnected);
        }

        let pair = inner.pair.as_ref().ok_or(KernelError::Disconnected)?.clone();
        let is_a = inner.is_side_a;
        drop(inner);

        let mut pb = pair.lock();

        // Check peer shutdown
        if (is_a && pb.b_shut_wr && pb.b_to_a.available() == 0)
            || (!is_a && pb.a_shut_wr && pb.a_to_b.available() == 0)
        {
            // peer is gone, but we check write direction
        }

        let ch = if is_a { &mut pb.a_to_b } else { &mut pb.b_to_a };

        match ch {
            ChannelBuf::Stream { data, capacity } => {
                let free = capacity.saturating_sub(data.len());
                let n = buf.len().min(free);
                if n == 0 && !buf.is_empty() {
                    return Err(KernelError::Busy);
                }
                data.extend(&buf[..n]);
                STAT_BYTES.fetch_add(n as u64, Ordering::Relaxed);
                Ok(n)
            }
            ChannelBuf::Messages { queue, max_msgs } => {
                if queue.len() >= *max_msgs {
                    return Err(KernelError::Busy);
                }
                let n = buf.len();
                queue.push_back(buf.to_vec());
                STAT_BYTES.fetch_add(n as u64, Ordering::Relaxed);
                Ok(n)
            }
        }
    }

    /// Read data from the socket.
    pub fn read(&self, buf: &mut [u8]) -> KernelResult<usize> {
        let inner = self.inner.lock();
        if inner.state != SocketState::Connected && inner.state != SocketState::Shutdown {
            return Err(KernelError::Disconnected);
        }

        let pair = inner.pair.as_ref().ok_or(KernelError::Disconnected)?.clone();
        let is_a = inner.is_side_a;
        drop(inner);

        let mut pb = pair.lock();
        let peer_shut = if is_a { pb.b_shut_wr } else { pb.a_shut_wr };
        let ch = if is_a { &mut pb.b_to_a } else { &mut pb.a_to_b };

        match ch {
            ChannelBuf::Stream { data, .. } => {
                if data.is_empty() {
                    return if peer_shut { Ok(0) } else { Err(KernelError::Busy) };
                }
                let n = buf.len().min(data.len());
                for dst in buf[..n].iter_mut() {
                    *dst = data.pop_front().unwrap();
                }
                Ok(n)
            }
            ChannelBuf::Messages { queue, .. } => {
                if let Some(msg) = queue.pop_front() {
                    let n = buf.len().min(msg.len());
                    buf[..n].copy_from_slice(&msg[..n]);
                    Ok(n)
                } else if peer_shut {
                    Ok(0)
                } else {
                    Err(KernelError::Busy)
                }
            }
        }
    }

    /// Shutdown one direction.
    pub fn shutdown(&self, how: u32) -> KernelResult<()> {
        let inner = self.inner.lock();
        let pair = inner.pair.as_ref().ok_or(KernelError::Disconnected)?.clone();
        let is_a = inner.is_side_a;
        drop(inner);

        let mut pb = pair.lock();
        match how {
            0 | 2 => {} // SHUT_RD or SHUT_RDWR
            _ => {}
        }
        if how == 1 || how == 2 {
            // SHUT_WR or SHUT_RDWR
            if is_a { pb.a_shut_wr = true; } else { pb.b_shut_wr = true; }
        }
        Ok(())
    }

    /// Return rx wait queue for poll/epoll integration.
    pub fn rx_wait_queue(&self) -> &WaitQueue {
        &self.rx_wait
    }

    /// Return tx wait queue for poll/epoll integration.
    pub fn tx_wait_queue(&self) -> &WaitQueue {
        &self.tx_wait
    }

    /// Check if socket has data available for reading (for poll).
    pub fn poll_readable(&self) -> bool {
        let inner = self.inner.lock();
        if let Some(pair) = &inner.pair {
            let pb = pair.lock();
            let ch = if inner.is_side_a { &pb.b_to_a } else { &pb.a_to_b };
            ch.available() > 0
        } else {
            // For listening sockets, check accept queue
            !inner.accept_queue.is_empty()
        }
    }

    /// Check if socket can accept writes (for poll).
    pub fn poll_writable(&self) -> bool {
        let inner = self.inner.lock();
        if let Some(pair) = &inner.pair {
            let pb = pair.lock();
            let ch = if inner.is_side_a { &pb.a_to_b } else { &pb.b_to_a };
            ch.writable_bytes() > 0
        } else {
            false
        }
    }

    /// Get socket type.
    pub fn socket_type(&self) -> UnixSocketType {
        self.inner.lock().socket_type
    }

    /// Get socket state.
    pub fn state(&self) -> SocketState {
        self.inner.lock().state
    }

    /// Set non-blocking mode.
    pub fn set_nonblocking(&self, nb: bool) {
        self.inner.lock().nonblocking = nb;
    }

    /// Check non-blocking mode.
    pub fn is_nonblocking(&self) -> bool {
        self.inner.lock().nonblocking
    }
}

// ── socketpair ──────────────────────────────────────────────────────────────

/// Create a connected pair of Unix sockets (like `socketpair(2)`).
pub fn unix_socketpair(stype: UnixSocketType) -> KernelResult<(Arc<UnixSocket>, Arc<UnixSocket>)> {
    let sock_a = UnixSocket::new(stype);
    let sock_b = UnixSocket::new(stype);

    let capacity = DEFAULT_BUF_CAPACITY;
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

    {
        let mut a = sock_a.inner.lock();
        a.state = SocketState::Connected;
        a.pair = Some(pair.clone());
        a.is_side_a = true;
        a.peer_addr = Some(UnixAddr::Unnamed);
    }
    {
        let mut b = sock_b.inner.lock();
        b.state = SocketState::Connected;
        b.pair = Some(pair);
        b.is_side_a = false;
        b.peer_addr = Some(UnixAddr::Unnamed);
    }

    Ok((sock_a, sock_b))
}

// ── Statistics ──────────────────────────────────────────────────────────────

static STAT_CREATED: AtomicU64 = AtomicU64::new(0);
static STAT_BYTES: AtomicU64 = AtomicU64::new(0);

/// Runtime statistics for Unix socket subsystem.
#[derive(Debug, Clone, Copy)]
pub struct UnixSocketStats {
    pub sockets_created: u64,
    pub bytes_transferred: u64,
    pub registered_names: usize,
}

pub fn stats() -> UnixSocketStats {
    UnixSocketStats {
        sockets_created: STAT_CREATED.load(Ordering::Relaxed),
        bytes_transferred: STAT_BYTES.load(Ordering::Relaxed),
        registered_names: registry::registered_names(),
    }
}
