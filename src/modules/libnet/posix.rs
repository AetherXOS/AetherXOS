#[cfg(feature = "network_transport")]
use crate::modules::libnet::{DatagramSocket as _, StreamSocket as _};
use crate::modules::vfs::types::File;
#[cfg(feature = "network_transport")]
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
#[cfg(feature = "network_transport")]
use spin::Mutex;
#[path = "posix/socket_flags.rs"]
mod socket_flags;
#[path = "posix/socket_runtime.rs"]
mod socket_runtime;
#[path = "posix/support.rs"]
mod support;
#[path = "posix/errno_wrappers.rs"]
mod errno_wrappers;
#[path = "posix/transport_core.rs"]
mod transport_core;
#[path = "posix/transport_io.rs"]
mod transport_io;
#[path = "posix/transport_meta.rs"]
mod transport_meta;
#[path = "posix/descriptor_ops.rs"]
mod descriptor_ops;
#[path = "posix/descriptor_wait.rs"]
mod descriptor_wait;

#[cfg(feature = "network_transport")]
use socket_flags::{
    apply_fd_flags, apply_shutdown, apply_socket_option, fd_flags_from_socket_flags,
    query_socket_option, socket_options_from_flags, SocketFlags,
};
#[cfg(feature = "network_transport")]
use socket_runtime::{
    alloc_ephemeral_port, clear_last_error, ensure_datagram_socket, ensure_posix_available,
    poll_transport_hint, set_last_error, set_socket_flags, socket_flags, with_socket,
    with_socket_mut, DatagramState, PosixSocket, SocketFile, StreamState,
};
pub use support::{
    into_errno, map_errno, AddressFamily, FcntlCmd, PosixErrno, PosixFdFlags, PosixIoctlCmd,
    PosixMsgFlags, PosixPollEvents, PosixPollFd, PosixSelectResult, PosixSockOpt, PosixSockOptVal,
    ShutdownHow, SocketAddrV4, SocketOption, SocketType,
};
#[cfg(feature = "network_transport")]
pub use errno_wrappers::{
    accept4_errno, accept_errno, bind_errno, close_errno, connect_errno, dup2_errno, dup_errno,
    fcntl_errno, fcntl_getfl_errno, fcntl_setfl_errno, getpeername_errno, getsockname_errno,
    getsockopt_errno, ioctl_errno, listen_errno, poll_errno, recv_errno, recv_with_flags_errno,
    recvfrom_errno, recvfrom_with_flags_errno, select_errno, send_errno, sendto_errno,
    setsockopt_errno, shutdown_errno, socket_errno,
};
#[cfg(feature = "network_transport")]
pub use transport_core::{
    accept, bind, connect, listen, socket,
};
#[cfg(feature = "network_transport")]
pub use transport_io::{recv, recv_with_flags, recvfrom, recvfrom_with_flags, send, sendto};
#[cfg(feature = "network_transport")]
pub use transport_meta::{close, getpeername, getsockname, shutdown};
#[cfg(feature = "network_transport")]
pub use descriptor_wait::{poll, select};
#[cfg(feature = "network_transport")]
pub use descriptor_ops::{
    accept4, dup, dup2, fcntl, fcntl_getfl, fcntl_setfl, getsockopt, ioctl, set_nonblocking,
    set_socket_option, setsockopt, socket_options,
};

#[derive(Debug, Clone)]
pub struct PosixRecvFrom {
    pub addr: SocketAddrV4,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PosixSocketOptions {
    pub nonblocking: bool,
    pub reuse_addr: bool,
    pub recv_timeout_retries: usize,
    pub send_timeout_retries: usize,
}

#[cfg(test)]
#[path = "posix/tests.rs"]
mod tests;
