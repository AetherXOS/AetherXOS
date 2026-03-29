use super::{map_net_errno, PosixAddressFamily, PosixErrno, PosixSocketAddrV4, PosixSocketType};

const SOCKETPAIR_BASE_PORT: u16 = 45_000;
const SOCKETPAIR_ALLOC_ATTEMPTS: usize = 512;

static NEXT_SOCKETPAIR_PORT: core::sync::atomic::AtomicU16 =
    core::sync::atomic::AtomicU16::new(SOCKETPAIR_BASE_PORT);

pub fn socketpair() -> Result<(u32, u32), PosixErrno> {
    socketpair_typed(PosixSocketType::Stream)
}

pub fn socketpair_typed(socket_type: PosixSocketType) -> Result<(u32, u32), PosixErrno> {
    match socket_type {
        PosixSocketType::Stream => socketpair_stream(),
        PosixSocketType::Datagram => socketpair_datagram(),
    }
}

fn socketpair_stream() -> Result<(u32, u32), PosixErrno> {
    let mut last_err = PosixErrno::AddrInUse;
    for _ in 0..SOCKETPAIR_ALLOC_ATTEMPTS {
        let port = NEXT_SOCKETPAIR_PORT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if port < SOCKETPAIR_BASE_PORT {
            NEXT_SOCKETPAIR_PORT.store(
                SOCKETPAIR_BASE_PORT.saturating_add(1),
                core::sync::atomic::Ordering::Relaxed,
            );
            continue;
        }

        let listener = crate::modules::libnet::posix_socket_errno(
            PosixAddressFamily::Inet,
            PosixSocketType::Stream,
        )
        .map_err(map_net_errno)?;

        if let Err(err) =
            crate::modules::libnet::posix_bind_errno(listener, PosixSocketAddrV4::localhost(port))
        {
            let _ = crate::modules::libnet::posix_close_errno(listener);
            last_err = map_net_errno(err);
            if matches!(last_err, PosixErrno::AddrInUse) {
                continue;
            }
            return Err(last_err);
        }

        if let Err(err) = crate::modules::libnet::posix_listen_errno(listener, 1) {
            let _ = crate::modules::libnet::posix_close_errno(listener);
            return Err(map_net_errno(err));
        }

        let client = match crate::modules::libnet::posix_socket_errno(
            PosixAddressFamily::Inet,
            PosixSocketType::Stream,
        ) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = crate::modules::libnet::posix_close_errno(listener);
                return Err(map_net_errno(err));
            }
        };

        if let Err(err) =
            crate::modules::libnet::posix_connect_errno(client, PosixSocketAddrV4::localhost(port))
        {
            let _ = crate::modules::libnet::posix_close_errno(client);
            let _ = crate::modules::libnet::posix_close_errno(listener);
            return Err(map_net_errno(err));
        }

        let peer = match crate::modules::libnet::posix_accept_errno(listener) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = crate::modules::libnet::posix_close_errno(client);
                let _ = crate::modules::libnet::posix_close_errno(listener);
                return Err(map_net_errno(err));
            }
        };

        let _ = crate::modules::libnet::posix_close_errno(listener);
        return Ok((client, peer));
    }

    Err(last_err)
}

fn socketpair_datagram() -> Result<(u32, u32), PosixErrno> {
    let mut last_err = PosixErrno::AddrInUse;
    for _ in 0..SOCKETPAIR_ALLOC_ATTEMPTS {
        let port_a = NEXT_SOCKETPAIR_PORT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let port_b = NEXT_SOCKETPAIR_PORT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if port_a < SOCKETPAIR_BASE_PORT || port_b < SOCKETPAIR_BASE_PORT || port_a == port_b {
            NEXT_SOCKETPAIR_PORT.store(
                SOCKETPAIR_BASE_PORT.saturating_add(2),
                core::sync::atomic::Ordering::Relaxed,
            );
            continue;
        }

        let a = crate::modules::libnet::posix_socket_errno(
            PosixAddressFamily::Inet,
            PosixSocketType::Datagram,
        )
        .map_err(map_net_errno)?;
        let b = match crate::modules::libnet::posix_socket_errno(
            PosixAddressFamily::Inet,
            PosixSocketType::Datagram,
        ) {
            Ok(fd) => fd,
            Err(err) => {
                let _ = crate::modules::libnet::posix_close_errno(a);
                return Err(map_net_errno(err));
            }
        };

        if let Err(err) =
            crate::modules::libnet::posix_bind_errno(a, PosixSocketAddrV4::localhost(port_a))
        {
            let _ = crate::modules::libnet::posix_close_errno(a);
            let _ = crate::modules::libnet::posix_close_errno(b);
            last_err = map_net_errno(err);
            if matches!(last_err, PosixErrno::AddrInUse) {
                continue;
            }
            return Err(last_err);
        }

        if let Err(err) =
            crate::modules::libnet::posix_bind_errno(b, PosixSocketAddrV4::localhost(port_b))
        {
            let _ = crate::modules::libnet::posix_close_errno(a);
            let _ = crate::modules::libnet::posix_close_errno(b);
            last_err = map_net_errno(err);
            if matches!(last_err, PosixErrno::AddrInUse) {
                continue;
            }
            return Err(last_err);
        }

        if let Err(err) =
            crate::modules::libnet::posix_connect_errno(a, PosixSocketAddrV4::localhost(port_b))
        {
            let _ = crate::modules::libnet::posix_close_errno(a);
            let _ = crate::modules::libnet::posix_close_errno(b);
            return Err(map_net_errno(err));
        }

        if let Err(err) =
            crate::modules::libnet::posix_connect_errno(b, PosixSocketAddrV4::localhost(port_a))
        {
            let _ = crate::modules::libnet::posix_close_errno(a);
            let _ = crate::modules::libnet::posix_close_errno(b);
            return Err(map_net_errno(err));
        }

        return Ok((a, b));
    }

    Err(last_err)
}
