use super::super::*;
use crate::hal::syscalls_consts::linux_nr;
use crate::modules::linux_compat::sys_dispatcher::SyscallDispFrame;

/// Dispatcher for Linux-specific syscall extensions.
pub fn dispatch_linux(
    nr: usize,
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> Option<usize> {
    match nr {
        linux_nr::EPOLL_CREATE1 => Some(sys_linux_epoll_create1(f.a1)),
        linux_nr::EPOLL_CTL => Some(sys_linux_epoll_ctl(f.fd1(), f.a2, f.fd3(), f.u4())),
        linux_nr::EPOLL_WAIT => Some(sys_linux_epoll_wait(f.fd1(), f.u2(), f.a3, f.a4 as i32)),
        linux_nr::GETRANDOM => Some(sys_linux_getrandom(f.u1(), f.a2, f.a3)),
        linux_nr::EVENTFD => Some(sys_linux_eventfd(f.a1 as u32, f.a2 as i32)),
        linux_nr::EVENTFD2 => Some(sys_linux_eventfd2(f.a1 as u32, f.a2 as i32)),
        linux_nr::TIMERFD_CREATE => Some(sys_linux_timerfd_create(f.a1, f.a2)),
        linux_nr::INOTIFY_INIT1 => Some(sys_linux_inotify_init1(f.a1 as i32)),
        linux_nr::CLONE => Some(sys_linux_clone(
            f.a1,
            f.u2(),
            f.u3(),
            f.u4(),
            f.a5,
            f.a6,
            frame.rip as usize,
            frame.rflags as usize,
        )),
        linux_nr::MEMFD_CREATE => Some(sys_linux_memfd_create(f.u1(), f.a2)),
        linux_nr::OPENAT2 => Some(sys_linux_openat2(f.fd1(), f.u2(), f.u3(), f.a4)),
        linux_nr::PRCTL => Some(sys_linux_prctl(f.a1, f.a2, f.a3, f.a4, f.a5)),
        _ => None,
    }
}
