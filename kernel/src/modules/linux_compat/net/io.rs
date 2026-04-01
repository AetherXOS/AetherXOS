use super::super::*;
use crate::kernel::syscalls::{with_user_read_bytes, with_user_write_bytes};

/// `recvfrom(2)` — Receive a message from a socket.
pub fn sys_linux_recvfrom(
    fd: Fd,
    buf_ptr: UserPtr<u8>,
    len: usize,
    flags: usize,
    addr_ptr: UserPtr<u8>,
    len_ptr: UserPtr<u32>,
) -> usize {
    if len == 0 {
        return 0;
    }
    crate::require_posix_net!((fd, buf_ptr, len, flags, addr_ptr, len_ptr) => {
        let msg_flags = crate::modules::libnet::PosixMsgFlags::from_bits_truncate(flags as u32);
        let res = crate::modules::libnet::posix_recvfrom_with_flags_errno(fd.as_u32(), msg_flags);
        match res {
            Ok(packet) => {
                let copy_len = core::cmp::min(len, packet.payload.len());
                if let Err(e) = with_user_write_bytes(buf_ptr.addr, copy_len, |dst| {
                    dst.copy_from_slice(&packet.payload[..copy_len]);
                    0
                }) { return e; }

                if !addr_ptr.is_null() && !len_ptr.is_null() {
                    let _ = write_sockaddr_in(addr_ptr.addr, len_ptr.addr, packet.addr);
                }
                copy_len
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `sendto(2)` — Send a message on a socket.
pub fn sys_linux_sendto(
    fd: Fd,
    buf_ptr: UserPtr<u8>,
    len: usize,
    _flags: usize,
    addr_ptr: UserPtr<u8>,
    addr_len: usize,
) -> usize {
    crate::require_posix_net!((fd, buf_ptr, len, _flags, addr_ptr, addr_len) => {
        let payload = match with_user_read_bytes(buf_ptr.addr, len, |src| src.to_vec()) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let res = if addr_ptr.is_null() {
            crate::modules::libnet::posix_send_errno(fd.as_u32(), &payload)
        } else {
            let addr = match read_sockaddr_in(addr_ptr.addr, addr_len) {
                Ok(v) => v,
                Err(e) => return e,
            };
            crate::modules::libnet::posix_sendto_errno(fd.as_u32(), addr, &payload)
        };

        match res {
            Ok(n) => n,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// Helper for recvmsg with scatter-gather and Cmsg support.
pub fn sys_linux_recvmsg(fd: Fd, msg_ptr: UserPtr<LinuxMsgHdr>, flags: usize) -> usize {
    crate::require_posix_net!((fd, msg_ptr, flags) => {
        let mut msg = match msg_ptr.read() { Ok(v) => v, Err(e) => return e };
        let mut iovs_raw = match read_user_iovec(msg.msg_iov as usize, msg.msg_iovlen as usize) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let mut bufs = alloc::vec::Vec::new();
        let mut iov_ptrs = alloc::vec::Vec::new();
        for iov in &mut iovs_raw {
            let mut buf = alloc::vec![0u8; iov.iov_len as usize];
            iov_ptrs.push(buf.as_mut_ptr());
            bufs.push(buf);
        }

        // Wrap for POSIX layer
        let mut slices: alloc::vec::Vec<&mut [u8]> = bufs.iter_mut().map(|b| b.as_mut_slice()).collect();
        let mut received_fds = alloc::vec::Vec::new();

        let msg_flags = crate::modules::libnet::PosixMsgFlags::from_bits_truncate(flags as u32);
        match crate::modules::posix::net::recvmsg_full(fd.as_u32(), &mut slices, &mut received_fds, msg_flags) {
            Ok(n) => {
                // Copy back scatter data
                for (i, iov) in iovs_raw.iter().enumerate() {
                    let _ = with_user_write_bytes(iov.iov_base as usize, bufs[i].len(), |dst| {
                        dst.copy_from_slice(&bufs[i]);
                        0
                    });
                }

                // Handle SCM_RIGHTS
                if !received_fds.is_empty() && msg.msg_control != 0 && msg.msg_controllen >= 16 {
                    let cmsg = LinuxCmsghdr {
                        cmsg_len: (core::mem::size_of::<LinuxCmsghdr>() + received_fds.len() * 4) as u64,
                        cmsg_level: 1, // SOL_SOCKET
                        cmsg_type: 1,  // SCM_RIGHTS
                    };
                    let cmsg_ptr = UserPtr::<LinuxCmsghdr>::new(msg.msg_control as usize);
                    let _ = cmsg_ptr.write(&cmsg);
                    let fd_ptr = UserPtr::<u32>::new(msg.msg_control as usize + core::mem::size_of::<LinuxCmsghdr>());
                    for (i, &rfd) in received_fds.iter().enumerate() {
                        let _ = fd_ptr.add(i).write(&rfd);
                    }
                    msg.msg_controllen = cmsg.cmsg_len;
                } else {
                    msg.msg_controllen = 0;
                }

                msg.msg_flags = 0;
                let _ = msg_ptr.write(&msg);
                n
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `sendmsg(2)`
pub fn sys_linux_sendmsg(fd: Fd, msg_ptr: UserPtr<LinuxMsgHdr>, flags: usize) -> usize {
    crate::require_posix_net!((fd, msg_ptr, flags) => {
        let msg = match msg_ptr.read() { Ok(v) => v, Err(e) => return e };
        let iovs = match read_user_iovec(msg.msg_iov as usize, msg.msg_iovlen as usize) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let mut payload_slices = alloc::vec::Vec::new();
        let mut payload_data = alloc::vec::Vec::new();
        for iov in iovs {
            let p = match with_user_read_bytes(iov.iov_base as usize, iov.iov_len as usize, |src| src.to_vec()) {
                Ok(v) => v,
                Err(e) => return e,
            };
            payload_data.push(p);
        }
        for p in &payload_data {
            payload_slices.push(p.as_slice());
        }

        let mut control_fds = alloc::vec::Vec::new();
        if msg.msg_control != 0 && msg.msg_controllen >= 16 {
            let cmsg_ptr = UserPtr::<LinuxCmsghdr>::new(msg.msg_control as usize);
            if let Ok(cmsg) = cmsg_ptr.read() {
                if cmsg.cmsg_level == 1 && cmsg.cmsg_type == 1 { // SOL_SOCKET, SCM_RIGHTS
                    let n_fds = (cmsg.cmsg_len as usize - core::mem::size_of::<LinuxCmsghdr>()) / 4;
                    let fd_ptr = UserPtr::<u32>::new(msg.msg_control as usize + core::mem::size_of::<LinuxCmsghdr>());
                    for i in 0..n_fds {
                        if let Ok(cfd) = fd_ptr.add(i).read() {
                            control_fds.push(cfd);
                        }
                    }
                }
            }
        }

        let msg_flags = crate::modules::libnet::PosixMsgFlags::from_bits_truncate(flags as u32);
        match crate::modules::posix::net::sendmsg_full(fd.as_u32(), &payload_slices, &control_fds, msg_flags) {
            Ok(n) => n,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `recvmmsg(2)` — Receive multiple messages at once (Modern High Performance).
pub fn sys_linux_recvmmsg(
    fd: Fd,
    mmsg_ptr: UserPtr<LinuxMmsghdr>,
    vlen: usize,
    flags: usize,
    _timeout_ptr: UserPtr<types::LinuxTimespec>,
) -> usize {
    crate::require_posix_net!((fd, mmsg_ptr, vlen, flags, _timeout_ptr) => {
        if vlen == 0 { return 0; }
        let mut woke = 0;
        for i in 0..vlen {
            let hdr_ptr = mmsg_ptr.add(i);
            // Re-use recvmsg logic for each sub-header.
            let res = sys_linux_recvmsg(fd, hdr_ptr.cast::<LinuxMsgHdr>(), flags);
            if (res as isize) < 0 {
                if woke == 0 { return res; } else { break; }
            }
            let mut mmsg = match hdr_ptr.read() { Ok(v) => v, Err(_) => break };
            mmsg.msg_len = res as u32;
            let _ = hdr_ptr.write(&mmsg);
            woke += 1;
        }
        woke
    })
}

/// `sendmmsg(2)` — Send multiple messages at once.
pub fn sys_linux_sendmmsg(
    fd: Fd,
    mmsg_ptr: UserPtr<LinuxMmsghdr>,
    vlen: usize,
    flags: usize,
) -> usize {
    crate::require_posix_net!((fd, mmsg_ptr, vlen, flags) => {
        let mut sent = 0;
        for i in 0..vlen {
            let hdr_ptr = mmsg_ptr.add(i);
            let res = sys_linux_sendmsg(fd, hdr_ptr.cast::<LinuxMsgHdr>(), flags);
            if (res as isize) < 0 {
                if sent == 0 { return res; } else { break; }
            }
            let mut mmsg = match hdr_ptr.read() { Ok(v) => v, Err(_) => break };
            mmsg.msg_len = res as u32;
            let _ = hdr_ptr.write(&mmsg);
            sent += 1;
        }
        sent
    })
}

pub fn sys_linux_shutdown(fd: Fd, how: usize) -> usize {
    crate::require_posix_net!((fd, how) => {
        let p_how = match crate::modules::libnet::PosixShutdownHow::from_raw(how as i32) {
            Some(v) => v,
            None => return linux_inval(),
        };
        match crate::modules::libnet::posix_shutdown_errno(fd.as_u32(), p_how) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}
