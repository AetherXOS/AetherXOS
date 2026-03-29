use super::super::super::*;
#[cfg(feature = "posix_net")]
use crate::kernel::syscalls::linux_shim::util::{read_user_pod, write_user_pod};
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_shim::util::define_user_pod_codec;

#[cfg(not(feature = "linux_compat"))]
pub(super) const LINUX_IOVEC_COMPAT_SIZE: usize = core::mem::size_of::<usize>() * 2;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxMsghdrCompat {
    name_ptr: usize,
    name_len: u32,
    _name_len_pad: u32,
    iov_ptr: usize,
    iov_len: usize,
    control_ptr: usize,
    control_len: usize,
    flags: i32,
    _flags_pad: u32,
}

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxIovecCompat {
    base: usize,
    len: usize,
}

#[cfg(not(feature = "linux_compat"))]
const LINUX_MSGHDR_NAMELEN_OFFSET: usize = core::mem::offset_of!(LinuxMsghdrCompat, name_len);
#[cfg(not(feature = "linux_compat"))]
const LINUX_MSGHDR_FLAGS_OFFSET: usize = core::mem::offset_of!(LinuxMsghdrCompat, flags);

#[cfg(not(feature = "linux_compat"))]
define_user_pod_codec!(read_linux_msghdr_pod, write_linux_msghdr_pod, LinuxMsghdrCompat);
#[cfg(not(feature = "linux_compat"))]
define_user_pod_codec!(read_linux_iovec_pod, _write_linux_iovec_pod, LinuxIovecCompat);

#[cfg(not(feature = "linux_compat"))]
pub(super) fn linux_shim_msg_iovec_cap_bytes() -> usize {
    const MIN_CAP: usize = 4096;
    const MAX_CAP: usize = 16 * 1024 * 1024;

    crate::config::KernelConfig::launch_max_boot_image_bytes().clamp(MIN_CAP, MAX_CAP)
}

#[cfg(feature = "posix_net")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct LinuxSockAddrInCompat {
    pub(super) sin_family: u16,
    pub(super) sin_port: u16,
    pub(super) sin_addr: [u8; 4],
    pub(super) sin_zero: [u8; 8],
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_linux_msghdr_compat(msg: usize) -> Result<(usize, usize, usize, usize), usize> {
    let hdr = read_linux_msghdr_pod(msg)?;
    Ok((
        hdr.name_ptr,
        hdr.name_len as usize,
        hdr.iov_ptr,
        hdr.iov_len,
    ))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_linux_iovec_compat(ptr: usize) -> Result<(usize, usize), usize> {
    let iov = read_linux_iovec_pod(ptr)?;
    Ok((iov.base, iov.len))
}

#[cfg(feature = "posix_net")]
pub(super) fn read_sockaddr_in_compat(
    ptr: usize,
    len: usize,
) -> Result<crate::modules::libnet::PosixSocketAddrV4, usize> {
    if len < core::mem::size_of::<LinuxSockAddrInCompat>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let tmp = read_user_pod::<LinuxSockAddrInCompat>(ptr)?;

    if i32::from(tmp.sin_family) != crate::modules::posix_consts::net::AF_INET {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    Ok(crate::modules::libnet::PosixSocketAddrV4 {
        addr: tmp.sin_addr,
        port: u16::from_be(tmp.sin_port),
    })
}

#[cfg(feature = "posix_net")]
pub(super) fn write_sockaddr_in_compat(
    ptr: usize,
    addr: crate::modules::libnet::PosixSocketAddrV4,
) -> usize {
    let sa = LinuxSockAddrInCompat {
        sin_family: crate::modules::posix_consts::net::AF_INET as u16,
        sin_port: addr.port.to_be(),
        sin_addr: addr.addr,
        sin_zero: [0; 8],
    };
    write_user_pod(ptr, &sa)
        .map(|_| 0usize)
        .unwrap_or_else(|err| err)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn write_linux_msghdr_namelen_compat(msg: usize, name_len: u32) -> Result<(), usize> {
    with_user_write_bytes(msg + LINUX_MSGHDR_NAMELEN_OFFSET, core::mem::size_of::<u32>(), |dst| {
        dst.copy_from_slice(&name_len.to_ne_bytes());
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn write_linux_msghdr_flags_compat(msg: usize, out_flags: i32) -> Result<(), usize> {
    with_user_write_bytes(msg + LINUX_MSGHDR_FLAGS_OFFSET, core::mem::size_of::<i32>(), |dst| {
        dst.copy_from_slice(&out_flags.to_ne_bytes());
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn write_recvmsg_result_metadata(
    msg: usize,
    name_len: u32,
    out_flags: i32,
) -> Result<(), usize> {
    write_linux_msghdr_namelen_compat(msg, name_len)?;
    write_linux_msghdr_flags_compat(msg, out_flags)
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn read_linux_msghdr_compat_invalid_pointer_returns_efault() {
        assert_eq!(
            read_linux_msghdr_compat(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn read_linux_iovec_compat_invalid_pointer_returns_efault() {
        assert_eq!(
            read_linux_iovec_compat(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn write_linux_msghdr_helpers_invalid_pointer_return_efault() {
        assert_eq!(
            write_linux_msghdr_namelen_compat(0, 16),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
        assert_eq!(
            write_linux_msghdr_flags_compat(0, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn recvmsg_metadata_writer_invalid_pointer_returns_efault() {
        assert_eq!(
            write_recvmsg_result_metadata(0, 16, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn read_sockaddr_in_compat_rejects_non_inet_family() {
        let sa = LinuxSockAddrInCompat {
            sin_family: crate::modules::posix_consts::net::AF_UNIX as u16,
            sin_port: 8080u16.to_be(),
            sin_addr: [127, 0, 0, 1],
            sin_zero: [0; 8],
        };
        assert_eq!(
            read_sockaddr_in_compat(
                (&sa as *const LinuxSockAddrInCompat) as usize,
                core::mem::size_of::<LinuxSockAddrInCompat>(),
            ),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn read_sockaddr_in_compat_rejects_short_length() {
        assert_eq!(
            read_sockaddr_in_compat(0, core::mem::size_of::<LinuxSockAddrInCompat>() - 1),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn write_sockaddr_in_compat_invalid_pointer_returns_efault() {
        let addr = crate::modules::libnet::PosixSocketAddrV4 {
            addr: [127, 0, 0, 1],
            port: 8080,
        };
        assert_eq!(
            write_sockaddr_in_compat(0, addr),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
}
