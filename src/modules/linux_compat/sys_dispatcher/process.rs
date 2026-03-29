use super::super::*;
use super::SyscallDispFrame;
use crate::hal::syscalls_consts::linux_nr;

pub fn dispatch_process(
    nr: usize,
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> Option<usize> {
    match nr {
        linux_nr::ARCH_PRCTL => Some(crate::modules::linux_compat::process::sys_linux_arch_prctl(
            f.a1, f.a2,
        )),
        linux_nr::BRK => Some(sys_linux_brk(f.a1)),

        // ── Thread/Process Lifecycle ──────────────────────────────────────────
        linux_nr::EXECVE => Some(crate::modules::linux_compat::process::sys_linux_execve(
            frame, f.a1, f.a2, f.a3,
        )),
        linux_nr::EXECVEAT => Some(crate::modules::linux_compat::sys::sys_linux_execveat(
            frame,
            f.fd1(),
            f.a2,
            f.a3,
            f.a4,
            f.a5,
        )),
        linux_nr::EXIT_GROUP => Some(crate::modules::linux_compat::process::sys_linux_exit_group(
            f.a1,
        )),

        // ── Process Info ─────────────────────────────────────────────────────
        linux_nr::GETEGID => Some(crate::modules::linux_compat::cred::sys_linux_getegid()),
        linux_nr::GETEUID => Some(crate::modules::linux_compat::cred::sys_linux_geteuid()),
        linux_nr::GETGID => Some(crate::modules::linux_compat::cred::sys_linux_getgid()),
        linux_nr::GETGROUPS => Some(crate::modules::linux_compat::cred::sys_linux_getgroups(
            f.a1,
            f.u2(),
        )),
        linux_nr::GETITIMER => Some(crate::modules::linux_compat::time::sys_linux_getitimer(
            f.a1,
            f.u2(),
        )),
        linux_nr::GETPGID => Some(crate::modules::linux_compat::cred::sys_linux_getpgid(f.a1)),
        linux_nr::GETPGRP => Some(crate::modules::linux_compat::cred::sys_linux_getpgrp()),
        linux_nr::GETCPU => Some(crate::modules::linux_compat::process::sys_linux_getcpu(
            f.u1(),
            f.u2(),
            f.u3(),
        )),
        linux_nr::GETPID => Some(crate::modules::linux_compat::cred::sys_linux_getpid()),
        linux_nr::GETPPID => Some(crate::modules::linux_compat::cred::sys_linux_getppid()),
        linux_nr::GETRESGID => Some(crate::modules::linux_compat::cred::sys_linux_getresgid(
            f.u1(),
            f.u2(),
            f.u3(),
        )),
        linux_nr::GETRESUID => Some(crate::modules::linux_compat::cred::sys_linux_getresuid(
            f.u1(),
            f.u2(),
            f.u3(),
        )),
        linux_nr::GETRLIMIT => Some(crate::modules::linux_compat::cred::sys_linux_getrlimit(
            f.a1, f.a2,
        )),
        linux_nr::GETRUSAGE => Some(crate::modules::linux_compat::process::sys_linux_getrusage(
            f.a1 as i32,
            f.u2(),
        )),
        linux_nr::GETSID => Some(crate::modules::linux_compat::cred::sys_linux_getsid(f.a1)),
        linux_nr::GETTID => Some(crate::modules::linux_compat::cred::sys_linux_gettid()),
        linux_nr::GETTIMEOFDAY => Some(crate::modules::linux_compat::time::sys_linux_gettimeofday(
            f.a1, f.a2,
        )),
        linux_nr::GETUID => Some(crate::modules::linux_compat::cred::sys_linux_getuid()),
        linux_nr::GETPRIORITY => Some(
            crate::modules::linux_compat::process::sys_linux_getpriority(f.a1 as i32, f.a2 as i32),
        ),
        linux_nr::SETPRIORITY => Some(
            crate::modules::linux_compat::process::sys_linux_setpriority(
                f.a1 as i32,
                f.a2 as i32,
                f.a3 as i32,
            ),
        ),
        linux_nr::SYSINFO => Some(crate::modules::linux_compat::sys::sys_linux_sysinfo(f.u1())),
        linux_nr::SYSLOG => Some(crate::modules::linux_compat::sys::sys_linux_syslog(
            f.a1,
            f.u2(),
            f.a3,
        )),
        linux_nr::TIMES => Some(crate::modules::linux_compat::sys::sys_linux_times(f.u1())),
        linux_nr::KILL => Some(crate::modules::linux_compat::sig::sys_linux_kill(
            f.a1, f.a2,
        )),

        // ── Memory Management ────────────────────────────────────────────────
        linux_nr::MADVISE => Some(crate::modules::linux_compat::mem::sys_linux_madvise(
            f.u1(),
            f.a2,
            f.a3,
        )),
        linux_nr::MLOCK => Some(crate::modules::linux_compat::mem::sys_linux_mlock(
            f.u1(),
            f.a2,
        )),
        linux_nr::MLOCKALL => Some(crate::modules::linux_compat::mem::sys_linux_mlockall(f.a1)),
        linux_nr::MREMAP => Some(crate::modules::linux_compat::mem::sys_linux_mremap(
            f.u1(),
            f.a2,
            f.a3,
            f.a4,
        )),
        linux_nr::MUNLOCK => Some(crate::modules::linux_compat::mem::sys_linux_munlock(
            f.u1(),
            f.a2,
        )),
        linux_nr::MUNLOCKALL => Some(crate::modules::linux_compat::mem::sys_linux_munlockall()),

        // ── Signals & Control ────────────────────────────────────────────────
        linux_nr::PRLIMIT64 => Some(crate::modules::linux_compat::cred::sys_linux_prlimit64(
            f.a1, f.a2, f.a3, f.a4,
        )),
        linux_nr::PERSONALITY => Some(crate::modules::linux_compat::cred::sys_linux_personality(
            f.a1,
        )),
        linux_nr::RT_SIGPENDING => Some(
            crate::modules::linux_compat::sig::sys_linux_rt_sigpending(f.u1(), f.a2),
        ),
        linux_nr::RT_SIGRETURN => Some(crate::modules::linux_compat::sig::sys_linux_rt_sigreturn(
            frame,
        )),
        linux_nr::RT_SIGSUSPEND => Some(
            crate::modules::linux_compat::sig::sys_linux_rt_sigsuspend(f.u1(), f.a2),
        ),
        linux_nr::SIGALTSTACK => Some(crate::modules::linux_compat::sig::sys_linux_sigaltstack(
            f.u1(),
            f.u2(),
        )),
        linux_nr::SIGNALFD => Some(crate::modules::linux_compat::sig::sys_linux_signalfd(
            f.fd1(),
            f.u2(),
            f.a3,
        )),
        linux_nr::SIGNALFD4 => Some(crate::modules::linux_compat::sig::sys_linux_signalfd4(
            f.fd1(),
            f.u2(),
            f.a3,
            f.a4 as i32,
        )),
        linux_nr::TGKILL => Some(crate::modules::linux_compat::sig::sys_linux_tgkill(
            f.a1, f.a2, f.a3,
        )),
        linux_nr::TKILL => Some(crate::modules::linux_compat::sig::sys_linux_tkill(
            f.a1, f.a2,
        )),

        // ── Scheduling ──────────────────────────────────────────────────────
        linux_nr::SCHED_GETAFFINITY => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_getaffinity(f.a1, f.a2, f.u3()))
        }
        linux_nr::SCHED_SETAFFINITY => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_setaffinity(f.a1, f.a2, f.u3()))
        }
        linux_nr::SETITIMER => Some(crate::modules::linux_compat::time::sys_linux_setitimer(
            f.a1,
            f.u2(),
            f.u3(),
        )),
        linux_nr::ALARM => Some(crate::modules::linux_compat::time::sys_linux_alarm(f.a1)),
        linux_nr::SETTIMEOFDAY => Some(crate::modules::linux_compat::time::sys_linux_settimeofday(
            f.u1(),
            f.u2(),
        )),
        linux_nr::SET_TID_ADDRESS => {
            Some(crate::modules::linux_compat::cred::sys_linux_set_tid_address(f.a1))
        }
        linux_nr::SETUID => Some(crate::modules::linux_compat::cred::sys_linux_setuid(f.a1)),
        linux_nr::SETGID => Some(crate::modules::linux_compat::cred::sys_linux_setgid(f.a1)),
        linux_nr::SETPGID => Some(crate::modules::linux_compat::cred::sys_linux_setpgid(
            f.a1, f.a2,
        )),
        linux_nr::SETSID => Some(crate::modules::linux_compat::cred::sys_linux_setsid()),
        linux_nr::SETGROUPS => Some(crate::modules::linux_compat::cred::sys_linux_setgroups(
            f.a1,
            f.u2(),
        )),
        linux_nr::SETRESUID => Some(crate::modules::linux_compat::cred::sys_linux_setresuid(
            f.a1, f.a2, f.a3,
        )),
        linux_nr::SETRESGID => Some(crate::modules::linux_compat::cred::sys_linux_setresgid(
            f.a1, f.a2, f.a3,
        )),
        linux_nr::SETREUID => Some(crate::modules::linux_compat::cred::sys_linux_setresuid(
            f.a1, f.a2, f.a2,
        )),
        linux_nr::SETREGID => Some(crate::modules::linux_compat::cred::sys_linux_setresgid(
            f.a1, f.a2, f.a2,
        )),
        linux_nr::SETRLIMIT => Some(crate::modules::linux_compat::cred::sys_linux_setrlimit(
            f.a1, f.a2,
        )),
        linux_nr::SETHOSTNAME => Some(crate::modules::linux_compat::sys::sys_linux_sethostname(
            f.u1(),
            f.a2,
        )),
        linux_nr::SETDOMAINNAME => Some(
            crate::modules::linux_compat::sys::sys_linux_setdomainname(f.u1(), f.a2),
        ),
        linux_nr::PAUSE => Some(crate::modules::linux_compat::sig::sys_linux_pause()),
        linux_nr::SCHED_YIELD => Some(crate::modules::linux_compat::sys::sys_linux_sched_yield()),
        linux_nr::SCHED_GETSCHEDULER => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_getscheduler(f.a1))
        }
        linux_nr::SCHED_SETSCHEDULER => Some(
            crate::modules::linux_compat::sys::sys_linux_sched_setscheduler(f.a1, f.a2, f.u3()),
        ),
        linux_nr::SCHED_GETPARAM => Some(
            crate::modules::linux_compat::sys::sys_linux_sched_getparam(f.a1, f.u2()),
        ),
        linux_nr::SCHED_SETPARAM => Some(
            crate::modules::linux_compat::sys::sys_linux_sched_setparam(f.a1, f.u2()),
        ),
        linux_nr::SCHED_GET_PRIORITY_MAX => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_get_priority_max(f.a1))
        }
        linux_nr::SCHED_GET_PRIORITY_MIN => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_get_priority_min(f.a1))
        }
        linux_nr::SCHED_RR_GET_INTERVAL => {
            Some(crate::modules::linux_compat::sys::sys_linux_sched_rr_get_interval(f.a1, f.u2()))
        }
        linux_nr::UNAME => Some(crate::modules::linux_compat::sys::sys_linux_uname(f.u1())),
        linux_nr::WAIT4 => Some(crate::modules::linux_compat::process::sys_linux_wait4(
            f.a1 as isize,
            f.u2(),
            f.a3,
            f.u4(),
        )),
        linux_nr::WAITID => Some(crate::modules::linux_compat::process::sys_linux_waitid(
            f.a1,
            f.a2,
            f.u3(),
            f.a4,
            f.u5(),
        )),
        linux_nr::UNSHARE => Some(crate::modules::linux_compat::process::sys_linux_unshare(f.a1)),
        linux_nr::SETNS => Some(crate::modules::linux_compat::process::sys_linux_setns(f.fd1(), f.a2)),
        _ => None,
    }
}
