use super::super::super::*;
use super::compat::*;
use super::message_support::{validate_iov_len, validate_linux_msg_flags};

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_ioctl(fd: usize, cmd: usize, arg: usize) -> usize {
    let _ = (fd, arg);
    match cmd {
        0x5421 => 0,
        _ => linux_errno(crate::modules::posix_consts::errno::ENOTTY),
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_sendmsg(fd: usize, msg: usize, flags: usize) -> usize {
    if let Err(err) = validate_linux_msg_flags(flags) {
        return err;
    }

    let (name_ptr, name_len, iov_ptr, iov_len) = match read_linux_msghdr_compat(msg) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if iov_len == 0 {
        return 0;
    }
    if let Err(err) = validate_iov_len(iov_len) {
        return err;
    }

    let mut payload = alloc::vec::Vec::new();
    let mut total_len = 0usize;
    let iovec_limit = linux_shim_msg_iovec_cap_bytes();
    for idx in 0..iov_len {
        let entry_ptr = match iov_ptr.checked_add(idx.saturating_mul(LINUX_IOVEC_COMPAT_SIZE)) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
        };
        let (base, len) = match read_linux_iovec_compat(entry_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if len == 0 {
            continue;
        }
        total_len = total_len.saturating_add(len);
        if total_len > iovec_limit {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let copy = with_user_read_bytes(base, len, |src| {
            payload.extend_from_slice(src);
            0usize
        });
        if copy.is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }
    }

    #[cfg(feature = "posix_net")]
    {
        if name_ptr == 0 {
            match crate::modules::libnet::posix_send_errno(fd as u32, &payload) {
                Ok(n) => n,
                Err(err) => linux_errno(err.code()),
            }
        } else {
            let addr = match read_sockaddr_in_compat(name_ptr, name_len) {
                Ok(v) => v,
                Err(e) => return e,
            };
            match crate::modules::libnet::posix_sendto_errno(fd as u32, addr, &payload) {
                Ok(n) => n,
                Err(err) => linux_errno(err.code()),
            }
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, name_ptr, name_len, iov_ptr, iov_len);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_recvmsg(fd: usize, msg: usize, flags: usize) -> usize {
    if let Err(err) = validate_linux_msg_flags(flags) {
        return err;
    }

    let (name_ptr, _name_len, iov_ptr, iov_len) = match read_linux_msghdr_compat(msg) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if iov_len == 0 {
        return 0;
    }
    if let Err(err) = validate_iov_len(iov_len) {
        return err;
    }

    let mut iovs = alloc::vec::Vec::new();
    let mut total_cap = 0usize;
    let iovec_limit = linux_shim_msg_iovec_cap_bytes();
    for idx in 0..iov_len {
        let entry_ptr = match iov_ptr.checked_add(idx.saturating_mul(LINUX_IOVEC_COMPAT_SIZE)) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
        };
        let (base, len) = match read_linux_iovec_compat(entry_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        total_cap = total_cap.saturating_add(len);
        if total_cap > iovec_limit {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        iovs.push((base, len));
    }
    if total_cap == 0 {
        return 0;
    }

    #[cfg(feature = "posix_net")]
    {
        let msg_flags = crate::modules::libnet::PosixMsgFlags::from_bits_truncate(flags as u32);
        let packet =
            match crate::modules::libnet::posix_recvfrom_with_flags_errno(fd as u32, msg_flags) {
                Ok(v) => v,
                Err(err) => return linux_errno(err.code()),
            };

        let mut copied = 0usize;
        for (base, len) in iovs {
            if copied >= packet.payload.len() {
                break;
            }
            if len == 0 {
                continue;
            }
            let take = core::cmp::min(len, packet.payload.len() - copied);
            let wr = with_user_write_bytes(base, take, |dst| {
                dst.copy_from_slice(&packet.payload[copied..copied + take]);
                take
            });
            if wr.is_err() {
                return linux_errno(crate::modules::posix_consts::errno::EFAULT);
            }
            copied += take;
        }

        if name_ptr != 0 {
            let wrote = write_sockaddr_in_compat(name_ptr, packet.addr);
            if wrote != 0 {
                return wrote;
            }
            if let Err(err) = write_recvmsg_result_metadata(
                msg,
                core::mem::size_of::<LinuxSockAddrInCompat>() as u32,
                0,
            ) {
                return err;
            }
        } else {
            if let Err(err) = write_recvmsg_result_metadata(msg, 0, 0) {
                return err;
            }
        }

        copied
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, msg, flags, name_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[repr(C)]
    struct TestMsgHdr {
        name: usize,
        namelen: u32,
        _pad0: u32,
        iov: usize,
        iovlen: usize,
        control: usize,
        controllen: usize,
        flags: u32,
        _pad1: u32,
    }

    #[repr(C)]
    #[cfg(feature = "posix_net")]
    struct TestIovec {
        base: usize,
        len: usize,
    }

    fn test_msghdr_ptr(iov: usize, iovlen: usize) -> usize {
        let msg = Box::leak(Box::new(TestMsgHdr {
            name: 0,
            namelen: 0,
            _pad0: 0,
            iov,
            iovlen,
            control: 0,
            controllen: 0,
            flags: 0,
            _pad1: 0,
        }));
        msg as *const TestMsgHdr as usize
    }

    #[cfg(feature = "posix_net")]
    fn test_msghdr_with_name_ptr(name: usize, namelen: u32, iov: usize, iovlen: usize) -> usize {
        let msg = Box::leak(Box::new(TestMsgHdr {
            name,
            namelen,
            _pad0: 0,
            iov,
            iovlen,
            control: 0,
            controllen: 0,
            flags: 0,
            _pad1: 0,
        }));
        msg as *const TestMsgHdr as usize
    }

    #[test_case]
    fn sendmsg_invalid_msghdr_pointer_returns_efault() {
        assert_eq!(
            sys_linux_sendmsg(0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn recvmsg_invalid_msghdr_pointer_returns_efault() {
        assert_eq!(
            sys_linux_recvmsg(0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn sendmsg_rejects_invalid_high_iovec_count_pointer_payload() {
        let bogus_msghdr = 1usize;
        assert_eq!(
            sys_linux_sendmsg(0, bogus_msghdr, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn sendmsg_zero_iov_returns_zero() {
        assert_eq!(sys_linux_sendmsg(0, test_msghdr_ptr(0, 0), 0), 0);
    }

    #[test_case]
    fn recvmsg_zero_iov_returns_zero() {
        assert_eq!(sys_linux_recvmsg(0, test_msghdr_ptr(0, 0), 0), 0);
    }

    #[test_case]
    fn sendmsg_rejects_excessive_iov_count_without_touching_iov_memory() {
        assert_eq!(
            sys_linux_sendmsg(0, test_msghdr_ptr(0, 1025), 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn recvmsg_rejects_excessive_iov_count_without_touching_iov_memory() {
        assert_eq!(
            sys_linux_recvmsg(0, test_msghdr_ptr(0, 1025), 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn sendmsg_nonzero_iov_with_null_iovec_pointer_returns_efault() {
        assert_eq!(
            sys_linux_sendmsg(0, test_msghdr_ptr(0, 1), 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn recvmsg_nonzero_iov_with_null_iovec_pointer_returns_efault() {
        assert_eq!(
            sys_linux_recvmsg(0, test_msghdr_ptr(0, 1), 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn socketpair_sendmsg_recvmsg_roundtrip_succeeds() {
        let mut sv = [0usize; 2];
        assert_eq!(
            crate::kernel::syscalls::linux_shim::net::socket::sys_linux_socketpair(
                crate::modules::posix_consts::net::AF_UNIX as usize,
                crate::modules::posix_consts::net::SOCK_STREAM as usize,
                0,
                sv.as_mut_ptr() as usize,
            ),
            0
        );

        let payload = *b"ping";
        let send_iov = TestIovec {
            base: payload.as_ptr() as usize,
            len: payload.len(),
        };
        let send_msg = test_msghdr_ptr((&send_iov as *const TestIovec) as usize, 1);
        assert_eq!(sys_linux_sendmsg(sv[0], send_msg, 0), payload.len());

        let mut recv_buf = [0u8; 8];
        let recv_iov = TestIovec {
            base: recv_buf.as_mut_ptr() as usize,
            len: recv_buf.len(),
        };
        let recv_msg = test_msghdr_ptr((&recv_iov as *const TestIovec) as usize, 1);
        assert_eq!(sys_linux_recvmsg(sv[1], recv_msg, 0), payload.len());
        assert_eq!(&recv_buf[..payload.len()], &payload);
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn recvmsg_writes_name_length_and_clears_flags_when_name_buffer_present() {
        let mut sv = [0usize; 2];
        assert_eq!(
            crate::kernel::syscalls::linux_shim::net::socket::sys_linux_socketpair(
                crate::modules::posix_consts::net::AF_UNIX as usize,
                crate::modules::posix_consts::net::SOCK_STREAM as usize,
                0,
                sv.as_mut_ptr() as usize,
            ),
            0
        );

        let payload = *b"ok";
        let send_iov = TestIovec {
            base: payload.as_ptr() as usize,
            len: payload.len(),
        };
        let send_msg = test_msghdr_ptr((&send_iov as *const TestIovec) as usize, 1);
        assert_eq!(sys_linux_sendmsg(sv[0], send_msg, 0), payload.len());

        let mut recv_buf = [0u8; 8];
        let recv_iov = TestIovec {
            base: recv_buf.as_mut_ptr() as usize,
            len: recv_buf.len(),
        };
        let mut name_buf = [0u8; core::mem::size_of::<LinuxSockAddrInCompat>()];
        let recv_msg = test_msghdr_with_name_ptr(
            name_buf.as_mut_ptr() as usize,
            core::mem::size_of::<LinuxSockAddrInCompat>() as u32,
            (&recv_iov as *const TestIovec) as usize,
            1,
        );

        assert_eq!(sys_linux_recvmsg(sv[1], recv_msg, 0), payload.len());
        assert_eq!(&recv_buf[..payload.len()], &payload);

        let (_, name_len, _, _) = read_linux_msghdr_compat(recv_msg).expect("msg header");
        assert_eq!(name_len, core::mem::size_of::<LinuxSockAddrInCompat>());
        let flags = with_user_read_bytes(recv_msg + 48, core::mem::size_of::<i32>(), |src| {
            i32::from_ne_bytes([src[0], src[1], src[2], src[3]])
        })
        .expect("flags");
        assert_eq!(flags, 0);
    }
}
