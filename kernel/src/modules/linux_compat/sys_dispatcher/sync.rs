use super::super::*;
use super::SyscallDispFrame;
use crate::hal::syscalls_consts::linux_nr;

pub fn dispatch_sync(nr: usize, f: &mut SyscallDispFrame) -> Option<usize> {
    match nr {
        linux_nr::EVENTFD => Some(sys_linux_eventfd(f.a1 as u32, f.a2 as i32)),
        linux_nr::EVENTFD2 => Some(sys_linux_eventfd2(f.a1 as u32, f.a2 as i32)),
        linux_nr::FUTEX => Some(sys_linux_futex(
            f.a1,
            f.a2,
            f.a3,
            f.u4::<u8>().cast(),
            f.a5,
            f.a6,
        )),
        linux_nr::FUTEX_WAITV => Some(sys_linux_futex_waitv(
            f.u1::<u8>().cast(),
            f.a2,
            f.a3,
            f.u4(),
        )),
        linux_nr::GET_ROBUST_LIST => Some(sys_linux_get_robust_list(f.a1 as i32, f.u2(), f.u3())),
        linux_nr::NANOSLEEP => Some(sys_linux_nanosleep(f.u1(), f.u2())),
        linux_nr::SET_ROBUST_LIST => Some(sys_linux_set_robust_list(f.a1, f.a2)),
        linux_nr::TIMERFD_CREATE => Some(sys_linux_timerfd_create(f.a1, f.a2)),
        linux_nr::TIMERFD_SETTIME => Some(sys_linux_timerfd_settime(f.fd1(), f.a2, f.u3(), f.u4())),
        linux_nr::TIMERFD_GETTIME => Some(sys_linux_timerfd_gettime(f.fd1(), f.u2())),
        _ => None,
    }
}
