use super::*;
use crate::modules::posix::epoll::EpollEvent;

pub fn sys_linux_eventfd(initval: u32) -> usize {
    sys_linux_eventfd2(initval, 0)
}

pub fn sys_linux_eventfd2(initval: u32, flags: i32) -> usize {
    crate::require_posix_fs!((initval, flags) => {
        match crate::modules::posix::io::eventfd_create_errno(initval, flags) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_epoll_create(size: i32) -> usize {
    sys_linux_epoll_create1(0)
}

pub fn sys_linux_epoll_create1(flags: i32) -> usize {
    if flags & !0x80000 != 0 { // Only O_CLOEXEC is allowed
        return linux_inval();
    }
    crate::require_posix_fs!((flags) => {
        match crate::modules::posix::epoll::epoll_create(0) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_epoll_ctl(epfd: Fd, op: i32, fd: Fd, event_ptr: UserPtr<EpollEvent>) -> usize {
    if event_ptr.is_null() && op != 2 { // op 2 is EPOLL_CTL_DEL
        return linux_fault();
    }
    
    let event = if !event_ptr.is_null() {
        match event_ptr.read() {
            Ok(e) => e,
            Err(e) => return e,
        }
    } else {
        EpollEvent { events: 0, data: 0 }
    };

    crate::require_posix_fs!((epfd, op, fd, event_ptr) => {
        match crate::modules::posix::epoll::epoll_ctl(epfd.as_u32(), op, fd.as_u32(), &event) {
            Ok(_) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_epoll_wait(
    epfd: Fd,
    events_ptr: UserPtr<EpollEvent>,
    maxevents: i32,
    timeout: i32,
) -> usize {
    if maxevents <= 0 || events_ptr.is_null() {
        return linux_inval();
    }

    crate::require_posix_fs!((epfd, events_ptr, maxevents, timeout) => {
        let mut buffer = alloc::vec![EpollEvent { events: 0, data: 0 }; maxevents as usize];
        match crate::modules::posix::epoll::epoll_wait(epfd.as_u32(), &mut buffer, timeout) {
            Ok(count) => {
                for i in 0..count {
                    if let Err(e) = events_ptr.add(i).write(&buffer[i]) {
                        return e;
                    }
                }
                count as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_epoll_pwait(
    epfd: Fd,
    events_ptr: UserPtr<EpollEvent>,
    maxevents: i32,
    timeout: i32,
    _sigmask: UserPtr<u8>,
    _sigsetsize: usize,
) -> usize {
    // Basic pwait implementation without sigmask support for now
    sys_linux_epoll_wait(epfd, events_ptr, maxevents, timeout)
}
