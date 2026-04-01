use super::super::super::*;
#[cfg(feature = "posix_net")]
use super::addr::{read_sockaddr_in, write_sockaddr_in};

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_sendto(
    fd: usize,
    buf_ptr: usize,
    len: usize,
    _flags: usize,
    addr_ptr: usize,
    addr_len: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        with_user_read_bytes(buf_ptr, len, |payload| {
            let res = if addr_ptr == 0 {
                crate::modules::libnet::posix_send_errno(fd as u32, payload)
            } else {
                let addr = match read_sockaddr_in(addr_ptr, addr_len) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                crate::modules::libnet::posix_sendto_errno(fd as u32, addr, payload)
            };

            match res {
                Ok(n) => n,
                Err(err) => linux_errno(err.code()),
            }
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, buf_ptr, len, _flags, addr_ptr, addr_len);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_recvfrom(
    fd: usize,
    buf_ptr: usize,
    len: usize,
    flags: usize,
    addr_ptr: usize,
    len_ptr: usize,
) -> usize {
    if len == 0 {
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

        let copy_len = core::cmp::min(len, packet.payload.len());
        let wr = with_user_write_bytes(buf_ptr, copy_len, |dst| {
            dst.copy_from_slice(&packet.payload[..copy_len]);
            copy_len
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
        if wr == linux_errno(crate::modules::posix_consts::errno::EFAULT) {
            return wr;
        }

        if addr_ptr != 0 && len_ptr != 0 {
            let sa = write_sockaddr_in(addr_ptr, len_ptr, packet.addr);
            if sa != 0 {
                return sa;
            }
        }

        copy_len
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, buf_ptr, len, flags, addr_ptr, len_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}
