#[cfg(feature = "posix_net")]
use super::super::super::*;
#[cfg(feature = "posix_net")]
use super::addr_support::{read_sockaddr_len, write_sockaddr_len};

#[cfg(feature = "posix_net")]
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct LinuxSockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: [u8; 4],
    pub sin_zero: [u8; 8],
}

#[cfg(feature = "posix_net")]
#[allow(dead_code)]
pub(super) fn read_sockaddr_in(
    ptr: usize,
    len: usize,
) -> Result<crate::modules::libnet::PosixSocketAddrV4, usize> {
    if len < core::mem::size_of::<LinuxSockAddrIn>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    with_user_read_bytes(ptr, core::mem::size_of::<LinuxSockAddrIn>(), |src| {
        let mut tmp = LinuxSockAddrIn {
            sin_family: 0,
            sin_port: 0,
            sin_addr: [0; 4],
            sin_zero: [0; 8],
        };
        let dst_ptr = &mut tmp as *mut LinuxSockAddrIn as *mut u8;
        let dst = unsafe {
            core::slice::from_raw_parts_mut(dst_ptr, core::mem::size_of::<LinuxSockAddrIn>())
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
#[allow(dead_code)]
pub(super) fn write_sockaddr_in(
    ptr: usize,
    len_ptr: usize,
    addr: crate::modules::libnet::PosixSocketAddrV4,
) -> usize {
    let want_len = core::mem::size_of::<LinuxSockAddrIn>();

    let given_len = match read_sockaddr_len(len_ptr) {
        Ok(len) => len,
        Err(err) => return err,
    };

    if given_len < want_len {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let rc = with_user_write_bytes(ptr, want_len, |dst| {
        let sa = LinuxSockAddrIn {
            sin_family: crate::modules::posix_consts::net::AF_INET as u16,
            sin_port: addr.port.to_be(),
            sin_addr: addr.addr,
            sin_zero: [0; 8],
        };
        let sa_ptr = &sa as *const LinuxSockAddrIn as *const u8;
        let sa_bytes = unsafe { core::slice::from_raw_parts(sa_ptr, want_len) };
        dst.copy_from_slice(sa_bytes);
        0usize
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));

    if rc != 0 {
        return rc;
    }

    write_sockaddr_len(len_ptr, want_len)
}

#[cfg(all(test, feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn read_sockaddr_in_rejects_short_length() {
        assert_eq!(
            read_sockaddr_in(0, core::mem::size_of::<LinuxSockAddrIn>() - 1),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn read_sockaddr_in_invalid_pointer_returns_efault() {
        assert_eq!(
            read_sockaddr_in(0, core::mem::size_of::<LinuxSockAddrIn>()),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn read_sockaddr_in_rejects_non_inet_family() {
        let sa = LinuxSockAddrIn {
            sin_family: crate::modules::posix_consts::net::AF_UNIX as u16,
            sin_port: 8080u16.to_be(),
            sin_addr: [127, 0, 0, 1],
            sin_zero: [0; 8],
        };
        assert_eq!(
            read_sockaddr_in(
                (&sa as *const LinuxSockAddrIn) as usize,
                core::mem::size_of::<LinuxSockAddrIn>(),
            ),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn write_sockaddr_in_invalid_length_pointer_returns_efault() {
        let addr = crate::modules::libnet::PosixSocketAddrV4 {
            addr: [127, 0, 0, 1],
            port: 8080,
        };
        assert_eq!(
            write_sockaddr_in(0, 0, addr),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn write_sockaddr_in_rejects_too_small_caller_buffer() {
        let addr = crate::modules::libnet::PosixSocketAddrV4 {
            addr: [127, 0, 0, 1],
            port: 8080,
        };
        let mut len = (core::mem::size_of::<LinuxSockAddrIn>() as u32) - 1;
        let mut out = [0u8; core::mem::size_of::<LinuxSockAddrIn>()];
        assert_eq!(
            write_sockaddr_in(
                out.as_mut_ptr() as usize,
                (&mut len as *mut u32) as usize,
                addr,
            ),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn write_sockaddr_in_successfully_writes_addr_and_updates_length() {
        let addr = crate::modules::libnet::PosixSocketAddrV4 {
            addr: [127, 0, 0, 1],
            port: 8080,
        };
        let mut len = core::mem::size_of::<LinuxSockAddrIn>() as u32;
        let mut out = [0u8; core::mem::size_of::<LinuxSockAddrIn>()];
        assert_eq!(
            write_sockaddr_in(
                out.as_mut_ptr() as usize,
                (&mut len as *mut u32) as usize,
                addr,
            ),
            0
        );
        assert_eq!(len as usize, core::mem::size_of::<LinuxSockAddrIn>());
        let written = unsafe { &*(out.as_ptr() as *const LinuxSockAddrIn) };
        assert_eq!(
            written.sin_family,
            crate::modules::posix_consts::net::AF_INET as u16
        );
        assert_eq!(written.sin_addr, [127, 0, 0, 1]);
        assert_eq!(u16::from_be(written.sin_port), 8080);
    }
}
