mod epoll;
mod msg;
mod socket;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_create(size: usize) -> usize {
    epoll::sys_linux_epoll_create(size)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_create1(flags: usize) -> usize {
    epoll::sys_linux_epoll_create1(flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_ctl(epfd: usize, op: usize, fd: usize, event_ptr: usize) -> usize {
    epoll::sys_linux_epoll_ctl(epfd, op, fd, event_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_pwait(
    epfd: usize,
    events_ptr: usize,
    maxevents: usize,
    timeout: usize,
    sigmask_ptr: usize,
    sigset_size: usize,
) -> usize {
    epoll::sys_linux_epoll_pwait(
        epfd,
        events_ptr,
        maxevents,
        timeout,
        sigmask_ptr,
        sigset_size,
    )
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_pwait2(
    epfd: usize,
    events_ptr: usize,
    maxevents: usize,
    timeout_ptr: usize,
    sigmask_ptr: usize,
    sigset_size: usize,
) -> usize {
    epoll::sys_linux_epoll_pwait2(
        epfd,
        events_ptr,
        maxevents,
        timeout_ptr,
        sigmask_ptr,
        sigset_size,
    )
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_ioctl(fd: usize, cmd: usize, arg: usize) -> usize {
    msg::sys_linux_ioctl(fd, cmd, arg)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sendmsg(fd: usize, msg_ptr: usize, flags: usize) -> usize {
    msg::sys_linux_sendmsg(fd, msg_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_recvmsg(fd: usize, msg_ptr: usize, flags: usize) -> usize {
    msg::sys_linux_recvmsg(fd, msg_ptr, flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_accept(
    fd: usize,
    addr_ptr: usize,
    len_ptr: usize,
    flags_raw: i32,
) -> usize {
    socket::sys_linux_accept(fd, addr_ptr, len_ptr, flags_raw)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_bind(fd: usize, addr_ptr: usize, addr_len: usize) -> usize {
    socket::sys_linux_bind(fd, addr_ptr, addr_len)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_connect(fd: usize, addr_ptr: usize, addr_len: usize) -> usize {
    socket::sys_linux_connect(fd, addr_ptr, addr_len)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getpeername(fd: usize, addr_ptr: usize, len_ptr: usize) -> usize {
    socket::sys_linux_getpeername(fd, addr_ptr, len_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getsockname(fd: usize, addr_ptr: usize, len_ptr: usize) -> usize {
    socket::sys_linux_getsockname(fd, addr_ptr, len_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval_ptr: usize,
    optlen_ptr: usize,
) -> usize {
    socket::sys_linux_getsockopt(fd, level, optname, optval_ptr, optlen_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_listen(fd: usize, backlog: usize) -> usize {
    socket::sys_linux_listen(fd, backlog)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_recvfrom(
    fd: usize,
    buf_ptr: usize,
    len: usize,
    flags: usize,
    addr_ptr: usize,
    len_ptr: usize,
) -> usize {
    socket::sys_linux_recvfrom(fd, buf_ptr, len, flags, addr_ptr, len_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sendto(
    fd: usize,
    buf_ptr: usize,
    len: usize,
    flags: usize,
    addr_ptr: usize,
    addr_len: usize,
) -> usize {
    socket::sys_linux_sendto(fd, buf_ptr, len, flags, addr_ptr, addr_len)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_setsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval_ptr: usize,
    optlen: usize,
) -> usize {
    socket::sys_linux_setsockopt(fd, level, optname, optval_ptr, optlen)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_shutdown(fd: usize, how: usize) -> usize {
    socket::sys_linux_shutdown(fd, how)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_socket(domain: usize, sock_type: usize, protocol: usize) -> usize {
    socket::sys_linux_socket(domain, sock_type, protocol)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_socketpair(
    domain: usize,
    sock_type: usize,
    protocol: usize,
    sv_ptr: usize,
) -> usize {
    socket::sys_linux_socketpair(domain, sock_type, protocol, sv_ptr)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn userspace_display_poll_revents(fd: u32, requested: u16) -> u16 {
    #[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
    {
        return socket::userspace_display_poll_revents(fd, requested);
    }

    #[cfg(not(all(feature = "posix_net", feature = "linux_userspace_graphics")))]
    {
        let _ = (fd, requested);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn userspace_display_fd_is_bound(fd: u32) -> bool {
    #[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
    {
        return socket::userspace_display_fd_is_bound(fd);
    }

    #[cfg(not(all(feature = "posix_net", feature = "linux_userspace_graphics")))]
    {
        let _ = fd;
        false
    }
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn userspace_display_epoll_revents(fd: u32, requested: u32) -> u32 {
    let timerfd_revents = crate::kernel::syscalls::linux_misc::timerfd_poll_revents(
        fd,
        requested as u16,
    ) as u32;

    #[cfg(all(feature = "posix_net", feature = "linux_userspace_graphics"))]
    {
        return socket::userspace_display_epoll_revents(fd, requested) | timerfd_revents;
    }

    #[cfg(not(all(feature = "posix_net", feature = "linux_userspace_graphics")))]
    {
        let _ = (fd, requested);
        timerfd_revents
    }
}
