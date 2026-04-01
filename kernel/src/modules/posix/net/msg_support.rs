use super::{
    map_net_errno, posix_recv_with_flags_errno, posix_recvfrom_with_flags_errno, PosixErrno,
    PosixMsgFlags,
};

pub fn sendmsg(fd: u32, iov: &[&[u8]]) -> Result<usize, PosixErrno> {
    sendmsg_flags(fd, iov, PosixMsgFlags::empty())
}

pub fn sendmsg_full(
    fd: u32,
    iov: &[&[u8]],
    control_fds: &[u32],
    flags: PosixMsgFlags,
) -> Result<usize, PosixErrno> {
    let sent = sendmsg_flags(fd, iov, flags)?;

    if !control_fds.is_empty() {
        // Find target port if it's a Unix socket
        // This is a bit of a hack since we don't know the peer directly here,
        // but we can look it up in libnet's connection state or assume directed.
        // For simulation: we assume the socket is connected and find its peer.
        if let Ok(peer) = crate::modules::libnet::posix_getpeername_errno(fd) {
            let mut files = alloc::vec::Vec::new();
            for &cfd in control_fds {
                if let Ok(shared) = crate::modules::posix::fs::get_file_description(cfd) {
                    files.push(shared);
                }
            }
            if !files.is_empty() {
                super::ancillary::push_rights(peer.port, files);
            }
        }
    }

    Ok(sent)
}

pub fn recvmsg(fd: u32, iov: &mut [&mut [u8]]) -> Result<usize, PosixErrno> {
    recvmsg_flags(fd, iov, PosixMsgFlags::empty())
}

pub fn recvmsg_full(
    fd: u32,
    iov: &mut [&mut [u8]],
    out_fds: &mut alloc::vec::Vec<u32>,
    flags: PosixMsgFlags,
) -> Result<usize, PosixErrno> {
    let n = recvmsg_flags(fd, iov, flags)?;

    if n > 0 {
        // Check for in-flight rights
        if let Ok(me) = crate::modules::libnet::posix_getsockname_errno(fd) {
            if let Some(packet) = super::ancillary::pop_rights(me.port) {
                for file in packet.files {
                    let new_fd = crate::modules::posix::fs::register_file_description(file);
                    out_fds.push(new_fd);
                }
            }
        }
    }

    Ok(n)
}

pub fn recvfrom_flags(
    fd: u32,
    flags: PosixMsgFlags,
) -> Result<crate::modules::libnet::PosixRecvFrom, PosixErrno> {
    posix_recvfrom_with_flags_errno(fd, flags).map_err(map_net_errno)
}

pub fn sendmsg_flags(fd: u32, iov: &[&[u8]], flags: PosixMsgFlags) -> Result<usize, PosixErrno> {
    if flags.contains(PosixMsgFlags::PEEK) {
        return Err(PosixErrno::Invalid);
    }

    let total_len = iov
        .iter()
        .fold(0usize, |acc, part| acc.saturating_add(part.len()));
    if total_len == 0 {
        return Ok(0);
    }

    let mut payload = alloc::vec::Vec::with_capacity(total_len);
    for part in iov {
        payload.extend_from_slice(part);
    }

    crate::modules::libnet::posix_send_errno(fd, &payload).map_err(map_net_errno)
}

pub fn recvmsg_flags(
    fd: u32,
    iov: &mut [&mut [u8]],
    flags: PosixMsgFlags,
) -> Result<usize, PosixErrno> {
    let payload = posix_recv_with_flags_errno(fd, flags).map_err(map_net_errno)?;
    if payload.is_empty() {
        return Ok(0);
    }

    let mut copied = 0usize;
    for slot in iov.iter_mut() {
        if copied >= payload.len() {
            break;
        }
        let remaining = payload.len() - copied;
        let n = core::cmp::min(slot.len(), remaining);
        slot[..n].copy_from_slice(&payload[copied..copied + n]);
        copied += n;
    }
    Ok(copied)
}

pub fn sendmmsg(fd: u32, messages: &[&[u8]], flags: PosixMsgFlags) -> Result<usize, PosixErrno> {
    if flags.contains(PosixMsgFlags::PEEK) {
        return Err(PosixErrno::Invalid);
    }

    let mut sent_messages = 0usize;
    for msg in messages {
        if msg.is_empty() {
            sent_messages += 1;
            continue;
        }
        match sendmsg_flags(fd, core::slice::from_ref(msg), flags) {
            Ok(_) => sent_messages += 1,
            Err(err) => {
                if sent_messages > 0 {
                    return Ok(sent_messages);
                }
                return Err(err);
            }
        }
    }
    Ok(sent_messages)
}

pub fn sendmsg_async(fd: u32, iov: &[&[u8]]) -> Result<usize, PosixErrno> {
    sendmsg_flags(fd, iov, PosixMsgFlags::DONTWAIT | PosixMsgFlags::NOSIGNAL)
}

pub fn recvmsg_async(fd: u32, iov: &mut [&mut [u8]]) -> Result<usize, PosixErrno> {
    recvmsg_flags(fd, iov, PosixMsgFlags::DONTWAIT)
}

pub fn sendmmsg_async(fd: u32, messages: &[&[u8]]) -> Result<usize, PosixErrno> {
    sendmmsg(
        fd,
        messages,
        PosixMsgFlags::DONTWAIT | PosixMsgFlags::NOSIGNAL,
    )
}

pub fn recvmmsg_async(fd: u32, iovecs: &mut [&mut [u8]]) -> Result<usize, PosixErrno> {
    recvmmsg(fd, iovecs, PosixMsgFlags::DONTWAIT)
}

pub fn recvmmsg(
    fd: u32,
    iovecs: &mut [&mut [u8]],
    flags: PosixMsgFlags,
) -> Result<usize, PosixErrno> {
    let mut recv_messages = 0usize;
    for iov in iovecs.iter_mut() {
        match recvmsg_flags(fd, core::slice::from_mut(iov), flags) {
            Ok(0) => return Ok(recv_messages),
            Ok(_) => recv_messages += 1,
            Err(err) => {
                if recv_messages > 0 && matches!(err, PosixErrno::Again | PosixErrno::TimedOut) {
                    return Ok(recv_messages);
                }
                if recv_messages > 0 {
                    return Ok(recv_messages);
                }
                return Err(err);
            }
        }
    }
    Ok(recv_messages)
}
