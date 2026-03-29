use super::*;

#[test_case]
fn connect_invalid_sockaddr_pointer_returns_efault() {
    assert_eq!(
        sys_linux_connect(
            0,
            0,
            core::mem::size_of::<super::super::addr::LinuxSockAddrIn>(),
        ),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn bind_short_sockaddr_returns_einval() {
    assert_eq!(
        sys_linux_bind(
            0,
            0,
            core::mem::size_of::<super::super::addr::LinuxSockAddrIn>() - 1
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn accept_rejects_invalid_flags_before_touching_socket_state() {
    assert_eq!(
        sys_linux_accept(
            0,
            0,
            0,
            crate::modules::posix_consts::net::SOCK_NONBLOCK | 0x4000
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn shutdown_rejects_invalid_how() {
    assert_eq!(
        sys_linux_shutdown(0, usize::MAX),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn socketpair_invalid_output_pointer_returns_efault() {
    assert_eq!(
        sys_linux_socketpair(
            crate::modules::posix_consts::net::AF_UNIX as usize,
            crate::modules::posix_consts::net::SOCK_STREAM as usize,
            0,
            0,
        ),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn socketpair_successfully_writes_two_fds() {
    let mut sv = [usize::MAX; 2];
    assert_eq!(
        sys_linux_socketpair(
            crate::modules::posix_consts::net::AF_UNIX as usize,
            crate::modules::posix_consts::net::SOCK_STREAM as usize,
            0,
            sv.as_mut_ptr() as usize,
        ),
        0
    );
    assert!(sv[0] != usize::MAX);
    assert!(sv[1] != usize::MAX);
    assert_ne!(sv[0], sv[1]);
}

#[test_case]
fn connect_short_sockaddr_returns_einval() {
    assert_eq!(
        sys_linux_connect(
            0,
            0,
            core::mem::size_of::<super::super::addr::LinuxSockAddrIn>() - 1
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn listen_passthrough_preserves_backend_error_contract() {
    let result = sys_linux_listen(usize::MAX, 0);
    assert!(
        result == 0 || result >= linux_errno(crate::modules::posix_consts::errno::EPERM),
        "listen should either succeed or return a linux errno encoding"
    );
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_bridge_accepts_wayland_sockaddr_un() {
    let fd = 73usize;
    let mut raw = [0u8; 48];
    raw[..2].copy_from_slice(&(crate::modules::posix_consts::net::AF_UNIX as u16).to_ne_bytes());
    let path = b"/run/user/1000/wayland-0\0";
    raw[2..2 + path.len()].copy_from_slice(path);

    assert_eq!(
        sys_linux_connect_userspace_display_bridge(fd, raw.as_ptr() as usize, 2 + path.len()),
        Some(0)
    );
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_bridge_rejects_non_display_unix_target() {
    let fd = 74usize;
    let mut raw = [0u8; 40];
    raw[..2].copy_from_slice(&(crate::modules::posix_consts::net::AF_UNIX as u16).to_ne_bytes());
    let path = b"/tmp/not-a-display\0";
    raw[2..2 + path.len()].copy_from_slice(path);

    assert_eq!(
        sys_linux_connect_userspace_display_bridge(fd, raw.as_ptr() as usize, 2 + path.len()),
        Some(linux_errno(crate::modules::posix_consts::errno::EAFNOSUPPORT))
    );
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_bind_bridge_marks_fd_and_listen_succeeds() {
    let fd = 77usize;
    let mut raw = [0u8; 48];
    raw[..2].copy_from_slice(&(crate::modules::posix_consts::net::AF_UNIX as u16).to_ne_bytes());
    let path = b"/tmp/.X11-unix/X2\0";
    raw[2..2 + path.len()].copy_from_slice(path);

    assert_eq!(
        sys_linux_bind_userspace_display_bridge(fd, raw.as_ptr() as usize, 2 + path.len()),
        Some(0)
    );
    assert!(userspace_display_fd_is_bound(fd as u32));
    assert_eq!(sys_linux_listen(fd, 64), 0);
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_accept_bridge_reports_eagain_without_pending_client() {
    let fd = 79usize;
    USERSPACE_DISPLAY_BRIDGE.lock().bound_fds.insert(fd as u32);
    assert_eq!(
        sys_linux_accept(fd, 0, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EAGAIN)
    );
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_poll_and_epoll_helpers_expose_write_readiness() {
    let fd = 81u32;
    USERSPACE_DISPLAY_BRIDGE.lock().bound_fds.insert(fd);

    assert_eq!(
        userspace_display_poll_revents(fd, crate::modules::posix_consts::net::POLLOUT),
        crate::modules::posix_consts::net::POLLOUT
    );
    assert_eq!(
        userspace_display_epoll_revents(fd, crate::modules::posix_consts::net::EPOLLOUT),
        crate::modules::posix_consts::net::EPOLLOUT
    );
}

#[cfg(feature = "linux_userspace_graphics")]
#[test_case]
fn userspace_display_connect_enqueues_pending_accept_and_sets_pollin() {
    let listener_fd = 90usize;
    let client_fd = 91usize;

    let mut raw = [0u8; 48];
    raw[..2].copy_from_slice(&(crate::modules::posix_consts::net::AF_UNIX as u16).to_ne_bytes());
    let path = b"/tmp/.X11-unix/X7\0";
    raw[2..2 + path.len()].copy_from_slice(path);

    assert_eq!(
        sys_linux_bind_userspace_display_bridge(listener_fd, raw.as_ptr() as usize, 2 + path.len()),
        Some(0)
    );
    assert_eq!(sys_linux_listen(listener_fd, 16), 0);
    assert_eq!(
        sys_linux_connect_userspace_display_bridge(client_fd, raw.as_ptr() as usize, 2 + path.len()),
        Some(0)
    );

    assert!(userspace_display_pending_accepts(listener_fd as u32) > 0);
    assert_eq!(
        userspace_display_poll_revents(listener_fd as u32, crate::modules::posix_consts::net::POLLIN),
        crate::modules::posix_consts::net::POLLIN
    );

    let accepted = sys_linux_accept(listener_fd, 0, 0, 0);
    assert!(accepted > 0);
}
