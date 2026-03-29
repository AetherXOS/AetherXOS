use super::*;

#[cfg(feature = "network_transport")]
pub fn socket_errno(family: AddressFamily, ty: SocketType) -> Result<u32, PosixErrno> {
    into_errno(socket(family, ty))
}

#[cfg(feature = "network_transport")]
pub fn bind_errno(fd: u32, addr: SocketAddrV4) -> Result<(), PosixErrno> {
    into_errno(bind(fd, addr))
}

#[cfg(feature = "network_transport")]
pub fn listen_errno(fd: u32, backlog: usize) -> Result<(), PosixErrno> {
    into_errno(listen(fd, backlog))
}

#[cfg(feature = "network_transport")]
pub fn connect_errno(fd: u32, addr: SocketAddrV4) -> Result<(), PosixErrno> {
    into_errno(connect(fd, addr))
}

#[cfg(feature = "network_transport")]
pub fn accept_errno(fd: u32) -> Result<u32, PosixErrno> {
    into_errno(accept(fd))
}

#[cfg(feature = "network_transport")]
pub fn send_errno(fd: u32, payload: &[u8]) -> Result<usize, PosixErrno> {
    into_errno(send(fd, payload))
}

#[cfg(feature = "network_transport")]
pub fn recv_errno(fd: u32) -> Result<Vec<u8>, PosixErrno> {
    into_errno(recv(fd))
}

#[cfg(feature = "network_transport")]
pub fn recv_with_flags_errno(fd: u32, msg_flags: PosixMsgFlags) -> Result<Vec<u8>, PosixErrno> {
    into_errno(recv_with_flags(fd, msg_flags))
}

#[cfg(feature = "network_transport")]
pub fn sendto_errno(fd: u32, addr: SocketAddrV4, payload: &[u8]) -> Result<usize, PosixErrno> {
    into_errno(sendto(fd, addr, payload))
}

#[cfg(feature = "network_transport")]
pub fn recvfrom_errno(fd: u32) -> Result<PosixRecvFrom, PosixErrno> {
    into_errno(recvfrom(fd))
}

#[cfg(feature = "network_transport")]
pub fn recvfrom_with_flags_errno(
    fd: u32,
    msg_flags: PosixMsgFlags,
) -> Result<PosixRecvFrom, PosixErrno> {
    into_errno(recvfrom_with_flags(fd, msg_flags))
}

#[cfg(feature = "network_transport")]
pub fn getsockname_errno(fd: u32) -> Result<SocketAddrV4, PosixErrno> {
    into_errno(getsockname(fd))
}

#[cfg(feature = "network_transport")]
pub fn getpeername_errno(fd: u32) -> Result<SocketAddrV4, PosixErrno> {
    into_errno(getpeername(fd))
}

#[cfg(feature = "network_transport")]
pub fn shutdown_errno(fd: u32, how: ShutdownHow) -> Result<(), PosixErrno> {
    into_errno(shutdown(fd, how))
}

#[cfg(feature = "network_transport")]
pub fn close_errno(fd: u32) -> Result<(), PosixErrno> {
    into_errno(close(fd))
}

#[cfg(feature = "network_transport")]
pub fn poll_errno(fds: &mut [PosixPollFd], retries: usize) -> Result<usize, PosixErrno> {
    into_errno(poll(fds, retries))
}

#[cfg(feature = "network_transport")]
pub fn select_errno(
    read_fds: &[u32],
    write_fds: &[u32],
    except_fds: &[u32],
    retries: usize,
) -> Result<PosixSelectResult, PosixErrno> {
    into_errno(select(read_fds, write_fds, except_fds, retries))
}

#[cfg(feature = "network_transport")]
pub fn fcntl_errno(fd: u32, cmd: FcntlCmd) -> Result<PosixFdFlags, PosixErrno> {
    into_errno(fcntl(fd, cmd))
}

#[cfg(feature = "network_transport")]
pub fn fcntl_getfl_errno(fd: u32) -> Result<PosixFdFlags, PosixErrno> {
    into_errno(fcntl_getfl(fd))
}

#[cfg(feature = "network_transport")]
pub fn fcntl_setfl_errno(fd: u32, flags: PosixFdFlags) -> Result<(), PosixErrno> {
    into_errno(fcntl_setfl(fd, flags))
}

#[cfg(feature = "network_transport")]
pub fn setsockopt_errno(
    fd: u32,
    option: PosixSockOpt,
    value: PosixSockOptVal,
) -> Result<(), PosixErrno> {
    into_errno(setsockopt(fd, option, value))
}

#[cfg(feature = "network_transport")]
pub fn getsockopt_errno(fd: u32, option: PosixSockOpt) -> Result<PosixSockOptVal, PosixErrno> {
    into_errno(getsockopt(fd, option))
}

#[cfg(feature = "network_transport")]
pub fn dup_errno(fd: u32) -> Result<u32, PosixErrno> {
    into_errno(dup(fd))
}

#[cfg(feature = "network_transport")]
pub fn dup2_errno(oldfd: u32, newfd: u32) -> Result<u32, PosixErrno> {
    into_errno(dup2(oldfd, newfd))
}

#[cfg(feature = "network_transport")]
pub fn accept4_errno(fd: u32, flags: PosixFdFlags) -> Result<u32, PosixErrno> {
    into_errno(accept4(fd, flags))
}

#[cfg(feature = "network_transport")]
pub fn ioctl_errno(fd: u32, cmd: PosixIoctlCmd) -> Result<usize, PosixErrno> {
    into_errno(ioctl(fd, cmd))
}