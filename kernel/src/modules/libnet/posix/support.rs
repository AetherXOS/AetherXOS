use alloc::vec::Vec;
use bitflags::bitflags;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    Inet = crate::modules::posix_consts::net::AF_INET,
}

impl AddressFamily {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::net::AF_INET => Some(Self::Inet),
            _ => None,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream = crate::modules::posix_consts::net::SOCK_STREAM,
    Datagram = crate::modules::posix_consts::net::SOCK_DGRAM,
}

impl SocketType {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::net::SOCK_STREAM => Some(Self::Stream),
            crate::modules::posix_consts::net::SOCK_DGRAM => Some(Self::Datagram),
            _ => None,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownHow {
    Read = crate::modules::posix_consts::net::SHUT_RD,
    Write = crate::modules::posix_consts::net::SHUT_WR,
    Both = crate::modules::posix_consts::net::SHUT_RDWR,
}

impl ShutdownHow {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::net::SHUT_RD => Some(Self::Read),
            crate::modules::posix_consts::net::SHUT_WR => Some(Self::Write),
            crate::modules::posix_consts::net::SHUT_RDWR => Some(Self::Both),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketOption {
    NonBlocking,
    ReuseAddr,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PosixPollEvents: u16 {
        const IN = crate::modules::posix_consts::net::POLLIN;
        const OUT = crate::modules::posix_consts::net::POLLOUT;
        const ERR = crate::modules::posix_consts::net::POLLERR;
        const HUP = crate::modules::posix_consts::net::POLLHUP;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PosixFdFlags: u32 {
        const NONBLOCK = crate::modules::posix_consts::net::O_NONBLOCK;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PosixMsgFlags: u32 {
        const PEEK = crate::modules::posix_consts::net::MSG_PEEK;
        const DONTWAIT = crate::modules::posix_consts::net::MSG_DONTWAIT;
        const WAITALL = crate::modules::posix_consts::net::MSG_WAITALL;
        const TRUNC = crate::modules::posix_consts::net::MSG_TRUNC;
        const NOSIGNAL = crate::modules::posix_consts::net::MSG_NOSIGNAL;
        const CMSG_CLOEXEC = crate::modules::posix_consts::net::MSG_CMSG_CLOEXEC;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixErrno {
    Again,
    BadFileDescriptor,
    Invalid,
    NotConnected,
    AddrInUse,
    TimedOut,
    NotSupported,
    WouldBlock,
    Other,
}

impl PosixErrno {
    pub const fn code(self) -> i32 {
        match self {
            Self::Again => crate::modules::posix_consts::errno::EAGAIN,
            Self::BadFileDescriptor => crate::modules::posix_consts::errno::EBADF,
            Self::Invalid => crate::modules::posix_consts::errno::EINVAL,
            Self::NotConnected => crate::modules::posix_consts::errno::ENOTCONN,
            Self::AddrInUse => crate::modules::posix_consts::errno::EADDRINUSE,
            Self::TimedOut => crate::modules::posix_consts::errno::ETIMEDOUT,
            Self::NotSupported => crate::modules::posix_consts::errno::EOPNOTSUPP,
            Self::WouldBlock => crate::modules::posix_consts::errno::EAGAIN,
            Self::Other => crate::modules::posix_consts::errno::EIO,
        }
    }

    pub const fn from_code(code: i32) -> Self {
        match code {
            crate::modules::posix_consts::errno::EAGAIN => Self::Again,
            crate::modules::posix_consts::errno::EBADF => Self::BadFileDescriptor,
            crate::modules::posix_consts::errno::EINVAL => Self::Invalid,
            crate::modules::posix_consts::errno::ENOTCONN => Self::NotConnected,
            crate::modules::posix_consts::errno::EADDRINUSE => Self::AddrInUse,
            crate::modules::posix_consts::errno::ETIMEDOUT => Self::TimedOut,
            crate::modules::posix_consts::errno::EOPNOTSUPP => Self::NotSupported,
            _ => Self::Other,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixSockOpt {
    SocketType = crate::modules::posix_consts::net::SO_TYPE,
    NonBlocking = crate::modules::posix_consts::net::SO_HYPER_NONBLOCK,
    ReuseAddr = crate::modules::posix_consts::net::SO_REUSEADDR,
    RecvTimeout = crate::modules::posix_consts::net::SO_RCVTIMEO,
    SendTimeout = crate::modules::posix_consts::net::SO_SNDTIMEO,
    RecvTimeoutRetries = crate::modules::posix_consts::net::SO_HYPER_RCVTIMEO_RETRIES,
    SendTimeoutRetries = crate::modules::posix_consts::net::SO_HYPER_SNDTIMEO_RETRIES,
    SocketDomain = crate::modules::posix_consts::net::SO_DOMAIN,
    SocketError = crate::modules::posix_consts::net::SO_ERROR,
}

impl PosixSockOpt {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::net::SO_TYPE => Some(Self::SocketType),
            crate::modules::posix_consts::net::SO_HYPER_NONBLOCK => Some(Self::NonBlocking),
            crate::modules::posix_consts::net::SO_REUSEADDR => Some(Self::ReuseAddr),
            crate::modules::posix_consts::net::SO_RCVTIMEO => Some(Self::RecvTimeout),
            crate::modules::posix_consts::net::SO_SNDTIMEO => Some(Self::SendTimeout),
            crate::modules::posix_consts::net::SO_HYPER_RCVTIMEO_RETRIES => {
                Some(Self::RecvTimeoutRetries)
            }
            crate::modules::posix_consts::net::SO_HYPER_SNDTIMEO_RETRIES => {
                Some(Self::SendTimeoutRetries)
            }
            crate::modules::posix_consts::net::SO_DOMAIN => Some(Self::SocketDomain),
            crate::modules::posix_consts::net::SO_ERROR => Some(Self::SocketError),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixSockOptVal {
    Bool(bool),
    Usize(usize),
    Errno(PosixErrno),
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixIoctlCmd {
    FionRead = crate::modules::posix_consts::net::FIONREAD,
}

impl PosixIoctlCmd {
    pub const fn as_raw(self) -> u64 {
        self as u64
    }

    pub const fn from_raw(value: u64) -> Option<Self> {
        match value {
            crate::modules::posix_consts::net::FIONREAD => Some(Self::FionRead),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlCmd {
    GetFl,
    SetFl(PosixFdFlags),
}

#[derive(Debug, Clone, Copy)]
pub struct PosixPollFd {
    pub fd: u32,
    pub events: PosixPollEvents,
    pub revents: PosixPollEvents,
}

impl PosixPollFd {
    pub const fn new(fd: u32, events: PosixPollEvents) -> Self {
        Self {
            fd,
            events,
            revents: PosixPollEvents::empty(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PosixSelectResult {
    pub readable: Vec<u32>,
    pub writable: Vec<u32>,
    pub exceptional: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
    pub addr: [u8; 4],
    pub port: u16,
}

impl SocketAddrV4 {
    pub const fn localhost(port: u16) -> Self {
        Self {
            addr: [127, 0, 0, 1],
            port,
        }
    }
}

pub fn map_errno(error: &'static str) -> PosixErrno {
    match error {
        "would block" => PosixErrno::WouldBlock,
        "invalid socket fd" => PosixErrno::BadFileDescriptor,
        "invalid bind port" => PosixErrno::Invalid,
        "datagram socket already bound" => PosixErrno::AddrInUse,
        "socket not connected" => PosixErrno::NotConnected,
        "datagram peer not connected" => PosixErrno::NotConnected,
        "socket read side is shut down" => PosixErrno::NotConnected,
        "socket write side is shut down" => PosixErrno::NotConnected,
        "recvfrom requires datagram or connected stream socket" => PosixErrno::NotConnected,
        "sendto requires datagram or connected stream socket" => PosixErrno::NotConnected,
        "listening socket has no peer" => PosixErrno::NotConnected,
        "unbound socket has no peer" => PosixErrno::NotConnected,
        "unsupported address family" => PosixErrno::NotSupported,
        "listen requires stream socket" => PosixErrno::NotSupported,
        "accept requires listening stream socket" => PosixErrno::NotSupported,
        "connected stream cannot listen" => PosixErrno::NotSupported,
        "listening socket cannot connect" => PosixErrno::NotSupported,
        "stream socket must be bound before listen" => PosixErrno::Invalid,
        "invalid stream state for bind" => PosixErrno::Invalid,
        _ => PosixErrno::Other,
    }
}

pub fn into_errno<T>(result: Result<T, &'static str>) -> Result<T, PosixErrno> {
    result.map_err(map_errno)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn numeric_wrappers_roundtrip_and_preserve_known_constants() {
        assert_eq!(
            AddressFamily::from_raw(crate::modules::posix_consts::net::AF_INET),
            Some(AddressFamily::Inet)
        );
        assert_eq!(
            SocketType::from_raw(crate::modules::posix_consts::net::SOCK_STREAM),
            Some(SocketType::Stream)
        );
        assert_eq!(
            ShutdownHow::from_raw(crate::modules::posix_consts::net::SHUT_RDWR),
            Some(ShutdownHow::Both)
        );
        assert_eq!(
            PosixSockOpt::from_raw(crate::modules::posix_consts::net::SO_ERROR),
            Some(PosixSockOpt::SocketError)
        );
        assert_eq!(
            PosixIoctlCmd::from_raw(crate::modules::posix_consts::net::FIONREAD),
            Some(PosixIoctlCmd::FionRead)
        );
        assert_eq!(PosixIoctlCmd::from_raw(0), None);
    }

    #[test_case]
    fn errno_mapping_covers_common_socket_failures() {
        assert_eq!(map_errno("would block"), PosixErrno::WouldBlock);
        assert_eq!(
            map_errno("invalid socket fd"),
            PosixErrno::BadFileDescriptor
        );
        assert_eq!(
            map_errno("datagram socket already bound"),
            PosixErrno::AddrInUse
        );
        assert_eq!(
            map_errno("accept requires listening stream socket"),
            PosixErrno::NotSupported
        );
        assert_eq!(
            map_errno("stream socket must be bound before listen"),
            PosixErrno::Invalid
        );
        assert_eq!(map_errno("unknown"), PosixErrno::Other);
    }

    #[test_case]
    fn socket_addr_localhost_and_pollfd_constructor_are_stable() {
        let addr = SocketAddrV4::localhost(4242);
        let pollfd = PosixPollFd::new(7, PosixPollEvents::IN | PosixPollEvents::OUT);

        assert_eq!(addr.addr, [127, 0, 0, 1]);
        assert_eq!(addr.port, 4242);
        assert_eq!(pollfd.fd, 7);
        assert!(pollfd.events.contains(PosixPollEvents::IN));
        assert!(pollfd.events.contains(PosixPollEvents::OUT));
        assert!(pollfd.revents.is_empty());
    }
}
