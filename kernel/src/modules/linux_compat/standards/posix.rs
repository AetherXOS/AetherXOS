use super::super::*;
use crate::hal::syscalls_consts::linux_nr;
use crate::modules::linux_compat::sys_dispatcher::SyscallDispFrame;

/// Dispatcher for IEEE 1003.1 (POSIX) compliant syscalls.
pub fn dispatch_posix(nr: usize, f: &mut SyscallDispFrame) -> Option<usize> {
    match nr {
        linux_nr::MMAP => Some(sys_linux_mmap(f.u1(), f.a2, f.a3, f.a4, f.fd5(), f.a6)),
        linux_nr::MUNMAP => Some(sys_linux_munmap(f.u1(), f.a2)),
        linux_nr::MPROTECT => Some(sys_linux_mprotect(f.u1(), f.a2, f.a3)),
        linux_nr::RT_SIGACTION => Some(sys_linux_rt_sigaction(f.a1, f.u2(), f.u3(), f.a4)),
        linux_nr::RT_SIGPROCMASK => Some(sys_linux_rt_sigprocmask(f.a1, f.u2(), f.u3(), f.a4)),
        linux_nr::NANOSLEEP => Some(sys_linux_nanosleep(f.u1(), f.u2())),
        linux_nr::CLOCK_GETTIME => Some(sys_linux_clock_gettime(f.a1, f.u2())),
        linux_nr::CLOCK_GETRES => Some(sys_linux_clock_getres(f.a1, f.u2())),
        linux_nr::CLOCK_SETTIME => Some(sys_linux_clock_settime(f.a1, f.u2())),
        linux_nr::CLOCK_NANOSLEEP => Some(sys_linux_clock_nanosleep(f.a1, f.a2, f.u3(), f.u4())),
        linux_nr::TIME => Some(sys_linux_time(f.u1())),
        linux_nr::SELECT => Some(sys_linux_select(
            f.a1,
            f.u2::<u8>().cast(),
            f.u3::<u8>().cast(),
            f.u4::<u8>().cast(),
            f.u5::<u8>().cast(),
        )),
        linux_nr::PSELECT6 => Some(sys_linux_pselect6(
            f.a1,
            f.u2::<u8>().cast(),
            f.u3::<u8>().cast(),
            f.u4::<u8>().cast(),
            f.u5(),
            f.u6(),
        )),
        linux_nr::POLL => Some(sys_linux_poll(f.u1(), f.a2, f.a3 as i32)),
        linux_nr::SHMGET => Some(sys_linux_shmget(f.a1 as i32, f.a2, f.a3 as i32)),
        linux_nr::SHMAT => Some(sys_linux_shmat(f.a1 as i32, f.u2(), f.a3 as i32)),
        linux_nr::SHMDT => Some(sys_linux_shmdt(f.u1())),
        linux_nr::MQ_OPEN => Some(sys_linux_mq_open(f.u1(), f.a2 as i32, f.a3 as u32, f.u4())),
        linux_nr::MQ_UNLINK => Some(sys_linux_mq_unlink(f.u1())),
        _ => None,
    }
}
