use super::super::super::*;

#[cfg(not(feature = "linux_compat"))]
pub(super) const LINUX_MSGHDR_COMPAT_SIZE: usize = 56;
#[cfg(not(feature = "linux_compat"))]
pub(super) const LINUX_IOVEC_COMPAT_SIZE: usize = core::mem::size_of::<usize>() * 2;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn linux_shim_msg_iovec_cap_bytes() -> usize {
    const MIN_CAP: usize = 4096;
    const MAX_CAP: usize = 16 * 1024 * 1024;

    crate::config::KernelConfig::launch_max_boot_image_bytes().clamp(MIN_CAP, MAX_CAP)
}

#[cfg(feature = "posix_net")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct LinuxSockAddrInCompat {
    pub(super) sin_family: u16,
    pub(super) sin_port: u16,
    pub(super) sin_addr: [u8; 4],
    pub(super) sin_zero: [u8; 8],
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_linux_msghdr_compat(msg: usize) -> Result<(usize, usize, usize, usize), usize> {
    with_user_read_bytes(msg, LINUX_MSGHDR_COMPAT_SIZE, |src| {
        let name_ptr = read_ne_usize(src, 0);
        let name_len = read_ne_u32(src, 8) as usize;
        let iov_ptr = read_ne_usize(src, 16);
        let iov_len = read_ne_usize(src, 24);
        (name_ptr, name_len, iov_ptr, iov_len)
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_linux_iovec_compat(ptr: usize) -> Result<(usize, usize), usize> {
    with_user_read_bytes(ptr, LINUX_IOVEC_COMPAT_SIZE, |src| {
        let base = read_ne_usize(src, 0);
        let len = read_ne_usize(src, 8);
        (base, len)
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
fn read_ne_u32(src: &[u8], off: usize) -> u32 {
    let mut tmp = [0u8; 4];
    tmp.copy_from_slice(&src[off..off + 4]);
    u32::from_ne_bytes(tmp)
}

#[cfg(not(feature = "linux_compat"))]
fn read_ne_usize(src: &[u8], off: usize) -> usize {
    let mut tmp = [0u8; core::mem::size_of::<usize>()];
    tmp.copy_from_slice(&src[off..off + core::mem::size_of::<usize>()]);
    usize::from_ne_bytes(tmp)
}

#[cfg(feature = "posix_net")]
pub(super) fn read_sockaddr_in_compat(
    ptr: usize,
    len: usize,
) -> Result<crate::modules::libnet::PosixSocketAddrV4, usize> {
    if len < core::mem::size_of::<LinuxSockAddrInCompat>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    with_user_read_bytes(ptr, core::mem::size_of::<LinuxSockAddrInCompat>(), |src| {
        let mut tmp = LinuxSockAddrInCompat {
            sin_family: 0,
            sin_port: 0,
            sin_addr: [0; 4],
            sin_zero: [0; 8],
        };
        let dst_ptr = &mut tmp as *mut LinuxSockAddrInCompat as *mut u8;
        let dst = unsafe {
            core::slice::from_raw_parts_mut(dst_ptr, core::mem::size_of::<LinuxSockAddrInCompat>())
        };
        dst.copy_from_slice(src);

        if i32::from(tmp.sin_family) != crate::modules::posix_consts::net::AF_INET {
            return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }

        Ok(crate::modules::libnet::PosixSocketAddrV4 {
            addr: tmp.sin_addr,
            port: u16::from_be(tmp.sin_port),
        })
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?
}

#[cfg(feature = "posix_net")]
pub(super) fn write_sockaddr_in_compat(
    ptr: usize,
    addr: crate::modules::libnet::PosixSocketAddrV4,
) -> usize {
    let want_len = core::mem::size_of::<LinuxSockAddrInCompat>();
    with_user_write_bytes(ptr, want_len, |dst| {
        let sa = LinuxSockAddrInCompat {
            sin_family: crate::modules::posix_consts::net::AF_INET as u16,
            sin_port: addr.port.to_be(),
            sin_addr: addr.addr,
            sin_zero: [0; 8],
        };
        let sa_ptr = &sa as *const LinuxSockAddrInCompat as *const u8;
        let sa_bytes = unsafe { core::slice::from_raw_parts(sa_ptr, want_len) };
        dst.copy_from_slice(sa_bytes);
        0usize
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn write_linux_msghdr_namelen_compat(msg: usize, name_len: u32) -> Result<(), usize> {
    with_user_write_bytes(msg + 8, core::mem::size_of::<u32>(), |dst| {
        dst.copy_from_slice(&name_len.to_ne_bytes());
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn write_linux_msghdr_flags_compat(msg: usize, out_flags: i32) -> Result<(), usize> {
    with_user_write_bytes(msg + 48, core::mem::size_of::<i32>(), |dst| {
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
