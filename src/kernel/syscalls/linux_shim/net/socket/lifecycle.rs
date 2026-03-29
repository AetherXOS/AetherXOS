use super::super::super::*;
#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
use alloc::string::String;
#[cfg(feature = "posix_net")]
use super::addr::{read_sockaddr_in, write_sockaddr_in};
#[cfg(feature = "posix_net")]
use super::lifecycle_support::{accept_socket_with_flags, write_socketpair_fds};
#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
use lazy_static::lazy_static;
#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
use spin::Mutex;

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
#[derive(Default)]
struct UserspaceDisplayBridgeState {
    bound_fds: BTreeSet<u32>,
    listening_fds: BTreeSet<u32>,
    endpoint_by_fd: BTreeMap<u32, String>,
    listener_by_endpoint: BTreeMap<String, u32>,
    pending_accepts: BTreeMap<u32, usize>,
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
lazy_static! {
    static ref USERSPACE_DISPLAY_BRIDGE: Mutex<UserspaceDisplayBridgeState> =
        Mutex::new(UserspaceDisplayBridgeState::default());
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
fn read_unix_sockaddr_bytes(addr_ptr: usize, addr_len: usize) -> Result<Option<[u8; 110]>, usize> {
    const SOCKADDR_UN_MAX: usize = 110;
    if addr_len < 3 {
        return Ok(None);
    }

    let copy_len = core::cmp::min(addr_len, SOCKADDR_UN_MAX);
    let mut raw = [0u8; SOCKADDR_UN_MAX];
    let read_ok = with_user_read_bytes(addr_ptr, copy_len, |src| {
        raw[..copy_len].copy_from_slice(src);
        0usize
    });
    if read_ok.is_err() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let family = u16::from_ne_bytes([raw[0], raw[1]]) as i32;
    if family != crate::modules::posix_consts::net::AF_UNIX {
        return Ok(None);
    }

    Ok(Some(raw))
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
fn display_endpoint_from_sockaddr_bytes(raw: &[u8], addr_len: usize) -> Option<String> {
    let copy_len = core::cmp::min(addr_len, raw.len());
    let bytes = &raw[..copy_len];
    let probe = crate::modules::userspace_graphics::probe_sockaddr_un_display_target(bytes)?;
    if !probe.is_display_socket {
        return None;
    }

    Some(probe.endpoint.path)
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
fn sys_linux_connect_userspace_display_bridge(fd: usize, addr_ptr: usize, addr_len: usize) -> Option<usize> {
    let raw = match read_unix_sockaddr_bytes(addr_ptr, addr_len) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(err) => return Some(err),
    };

    let Some(endpoint) = display_endpoint_from_sockaddr_bytes(&raw, addr_len) else {
        return Some(linux_errno(crate::modules::posix_consts::errno::EAFNOSUPPORT));
    };

    let mut state = USERSPACE_DISPLAY_BRIDGE.lock();
    if let Some(listener_fd) = state.listener_by_endpoint.get(&endpoint).copied() {
        if state.listening_fds.contains(&listener_fd) {
            let pending = state.pending_accepts.entry(listener_fd).or_insert(0);
            *pending = pending.saturating_add(1);
        }
    }

    state.bound_fds.insert(fd as u32);
    Some(0)
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
fn sys_linux_bind_userspace_display_bridge(fd: usize, addr_ptr: usize, addr_len: usize) -> Option<usize> {
    let raw = match read_unix_sockaddr_bytes(addr_ptr, addr_len) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(err) => return Some(err),
    };

    let Some(endpoint) = display_endpoint_from_sockaddr_bytes(&raw, addr_len) else {
        return Some(linux_errno(crate::modules::posix_consts::errno::EAFNOSUPPORT));
    };

    let fd_u32 = fd as u32;
    let mut state = USERSPACE_DISPLAY_BRIDGE.lock();
    if let Some(old_endpoint) = state.endpoint_by_fd.insert(fd_u32, endpoint.clone()) {
        state.listener_by_endpoint.remove(&old_endpoint);
    }
    state.bound_fds.insert(fd_u32);
    state.pending_accepts.entry(fd_u32).or_insert(0);

    Some(0)
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
fn userspace_display_pending_accepts(fd: u32) -> usize {
    USERSPACE_DISPLAY_BRIDGE
        .lock()
        .pending_accepts
        .get(&fd)
        .copied()
        .unwrap_or(0)
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
pub(crate) fn userspace_display_fd_is_bound(fd: u32) -> bool {
    USERSPACE_DISPLAY_BRIDGE.lock().bound_fds.contains(&fd)
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
pub(crate) fn userspace_display_poll_revents(fd: u32, requested: u16) -> u16 {
    if !userspace_display_fd_is_bound(fd) {
        return 0;
    }

    let mut revents = 0u16;
    let pollin = crate::modules::posix_consts::net::POLLIN;
    let pollout = crate::modules::posix_consts::net::POLLOUT;
    if (requested & pollin) != 0 && userspace_display_pending_accepts(fd) > 0 {
        revents |= pollin;
    }
    if (requested & pollout) != 0 {
        revents |= pollout;
    }

    revents
}

#[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
pub(crate) fn userspace_display_epoll_revents(fd: u32, requested: u32) -> u32 {
    if !userspace_display_fd_is_bound(fd) {
        return 0;
    }

    let mut revents = 0u32;
    let epollin = crate::modules::posix_consts::net::EPOLLIN;
    let epollout = crate::modules::posix_consts::net::EPOLLOUT;
    if (requested & epollin) != 0 && userspace_display_pending_accepts(fd) > 0 {
        revents |= epollin;
    }
    if (requested & epollout) != 0 {
        revents |= epollout;
    }

    revents
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_socket(domain: usize, sock_type: usize, protocol: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::posix::net::socket_raw_errno(
            domain as i32,
            sock_type as i32,
            protocol as i32,
        ) {
            Ok(fd) => fd as usize,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (domain, sock_type, protocol);
        linux_errno(crate::modules::posix_consts::errno::EAFNOSUPPORT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_connect(fd: usize, addr_ptr: usize, addr_len: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        #[cfg(feature = "linux_userspace_graphics")]
        if let Some(result) = sys_linux_connect_userspace_display_bridge(fd, addr_ptr, addr_len) {
            return result;
        }

        let addr = match read_sockaddr_in(addr_ptr, addr_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match crate::modules::libnet::posix_connect_errno(fd as u32, addr) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, addr_ptr, addr_len);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_bind(fd: usize, addr_ptr: usize, addr_len: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        #[cfg(feature = "linux_userspace_graphics")]
        if let Some(result) = sys_linux_bind_userspace_display_bridge(fd, addr_ptr, addr_len) {
            return result;
        }

        let addr = match read_sockaddr_in(addr_ptr, addr_len) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match crate::modules::libnet::posix_bind_errno(fd as u32, addr) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, addr_ptr, addr_len);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_listen(fd: usize, backlog: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        #[cfg(feature = "linux_userspace_graphics")]
        if userspace_display_fd_is_bound(fd as u32) {
            let mut state = USERSPACE_DISPLAY_BRIDGE.lock();
            state.listening_fds.insert(fd as u32);
            if let Some(endpoint) = state.endpoint_by_fd.get(&(fd as u32)).cloned() {
                state.listener_by_endpoint.insert(endpoint, fd as u32);
            }
            let _ = backlog;
            return 0;
        }

        match crate::modules::libnet::posix_listen_errno(fd as u32, backlog) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, backlog);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_accept(
    fd: usize,
    addr_ptr: usize,
    len_ptr: usize,
    flags_raw: i32,
) -> usize {
    #[cfg(feature = "linux_userspace_graphics")]
    {
        let allowed = (crate::modules::posix_consts::net::SOCK_NONBLOCK
            | crate::modules::posix_consts::net::SOCK_CLOEXEC) as i32;
        if (flags_raw & !allowed) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
    }

    #[cfg(feature = "posix_net")]
    {
        #[cfg(feature = "linux_userspace_graphics")]
        if userspace_display_fd_is_bound(fd as u32) {
            let mut state = USERSPACE_DISPLAY_BRIDGE.lock();
            let pending = state.pending_accepts.entry(fd as u32).or_insert(0);
            if *pending == 0 {
                let _ = (addr_ptr, len_ptr, flags_raw);
                return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
            }
            *pending -= 1;
            drop(state);

            return match crate::modules::posix::net::socket_raw_errno(
                crate::modules::posix_consts::net::AF_UNIX,
                crate::modules::posix_consts::net::SOCK_STREAM,
                0,
            ) {
                Ok(new_fd) => new_fd as usize,
                Err(err) => linux_errno(err.code()),
            };
        }

        let new_fd = match accept_socket_with_flags(fd, flags_raw) {
            Ok(v) => v,
            Err(err) => return err,
        };

        if addr_ptr != 0 && len_ptr != 0 {
            if let Ok(peer) = crate::modules::libnet::posix_getpeername_errno(new_fd) {
                let wr = write_sockaddr_in(addr_ptr, len_ptr, peer);
                if wr != 0 {
                    return wr;
                }
            }
        }

        new_fd as usize
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, addr_ptr, len_ptr, flags_raw);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
#[path = "lifecycle/tests.rs"]
mod tests;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_shutdown(fd: usize, how: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let how = match crate::modules::libnet::PosixShutdownHow::from_raw(how as i32) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EINVAL),
        };
        match crate::modules::libnet::posix_shutdown_errno(fd as u32, how) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, how);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_socketpair(
    domain: usize,
    sock_type: usize,
    protocol: usize,
    sv_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
        let (fd0, fd1) = match crate::modules::posix::net::socketpair_raw_errno(
            domain as i32,
            sock_type as i32,
            protocol as i32,
        ) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        match write_socketpair_fds(sv_ptr, fd0, fd1) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (domain, sock_type, protocol, sv_ptr);
        linux_errno(crate::modules::posix_consts::errno::EAFNOSUPPORT)
    }
}
