use super::super::*;

pub fn sys_linux_mq_open(name: UserPtr<u8>, oflag: i32, _mode: u32, attr: UserPtr<usize>) -> usize {
    let name_str = match name.as_str() {
        Ok(s) => s,
        Err(_) => return linux_inval(),
    };

    let mut max_msgs = 0;
    let mut max_msgsize = 0;
    if !attr.is_null() {
        if let Ok(m) = attr.read() {
            max_msgs = m;
        }
        if let Ok(m) = attr.offset(1).read() {
            max_msgsize = m;
        }
    }

    match crate::modules::posix::mq::mq_open(name_str, oflag, max_msgs, max_msgsize) {
        Ok(fd) => fd as usize,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_mq_unlink(name: UserPtr<u8>) -> usize {
    let name_str = match name.as_str() {
        Ok(s) => s,
        Err(_) => return linux_inval(),
    };
    match crate::modules::posix::mq::mq_unlink(name_str) {
        Ok(()) => 0,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_mq_timedsend(
    fd: Fd,
    msg_ptr: UserPtr<u8>,
    msg_len: usize,
    _msg_prio: u32,
    _abs_timeout: UserPtr<usize>,
) -> usize {
    let mut buf = alloc::vec![0u8; msg_len];
    if msg_ptr.read_bytes(&mut buf).is_err() {
        return linux_inval();
    }

    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = match table.get(&fd.as_u32()) {
        Some(d) => d,
        None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
    };

    let mut handle = desc.file.handle.lock();
    match handle.write(&buf) {
        Ok(n) => n,
        Err(e) => {
            if e == "full" {
                linux_errno(crate::modules::posix_consts::errno::EAGAIN)
            } else {
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            }
        }
    }
}

pub fn sys_linux_mq_timedreceive(
    fd: Fd,
    msg_ptr: UserPtr<u8>,
    msg_len: usize,
    _msg_prio: UserPtr<u32>,
    _abs_timeout: UserPtr<usize>,
) -> usize {
    let mut buf = alloc::vec![0u8; msg_len];
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = match table.get(&fd.as_u32()) {
        Some(d) => d,
        None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
    };

    let mut handle = desc.file.handle.lock();
    match handle.read(&mut buf) {
        Ok(n) => {
            if msg_ptr.write_bytes(&buf[..n]).is_err() {
                return linux_inval();
            }
            n
        }
        Err(e) => {
            if e == "empty" {
                linux_errno(crate::modules::posix_consts::errno::EAGAIN)
            } else {
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            }
        }
    }
}

pub fn sys_linux_mq_getsetattr(
    _fd: Fd,
    new_attr: UserPtr<usize>,
    old_attr: UserPtr<usize>,
) -> usize {
    let _ = new_attr; // SET logic could go here
    if !old_attr.is_null() {
        // Return baseline mq_attr { mq_flags, mq_maxmsg, mq_msgsize, mq_curmsgs }.
        let attrs = [0usize, 10, 8192, 0];
        let _ = old_attr
            .write_bytes(unsafe { core::slice::from_raw_parts(attrs.as_ptr() as *const u8, 32) });
    }
    0
}
