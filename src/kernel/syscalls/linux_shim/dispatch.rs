use super::util::arg5_to_zero;
use super::*;

#[cfg(not(feature = "linux_compat"))]
fn linux_unknown_syscall_errno(syscall_id: usize) -> usize {
    // Privileged kernel-control syscalls should look permission-gated rather than missing.
    let errno = match syscall_id {
        linux_nr::PTRACE
        | linux_nr::MOUNT
        | linux_nr::UMOUNT2
        | linux_nr::SWAPON
        | linux_nr::SWAPOFF
        | linux_nr::REBOOT
        | linux_nr::INIT_MODULE
        | linux_nr::DELETE_MODULE
        | linux_nr::KEXEC_LOAD
        | linux_nr::FINIT_MODULE
        | linux_nr::KEXEC_FILE_LOAD
        | linux_nr::MOVE_MOUNT
        | linux_nr::FSMOUNT
        | linux_nr::MOUNT_SETATTR => crate::modules::posix_consts::errno::EPERM,
        _ => crate::modules::posix_consts::errno::ENOSYS,
    };
    linux_errno(errno)
}

pub(super) fn sys_linux_shim(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    frame_ptr: *mut crate::kernel::syscalls::SyscallFrame,
) -> Option<usize> {
    let _ = arg6;

    if let Some(ret) = dispatch_socket_syscalls(syscall_id, arg1, arg2, arg3, arg4, arg5, arg6) {
        return Some(ret);
    }

    match syscall_id {
        linux_nr::SCHED_YIELD => Some(super::sys_yield()),
        linux_nr::EXIT | linux_nr::EXIT_GROUP => Some(super::sys_exit(arg1)),
        linux_nr::READ => Some(fs::sys_linux_read(arg1, arg2, arg3)),
        linux_nr::WRITE => Some(fs::sys_linux_write(arg1, arg2, arg3)),
        linux_nr::CLOSE => Some(fs::sys_linux_close(arg1)),
        linux_nr::LSEEK => Some(fs::sys_linux_lseek(arg1, arg2 as i64, arg3)),
        linux_nr::GETPID => Some(task_time::sys_linux_getpid()),
        linux_nr::GETPPID => Some(task_time::sys_linux_getppid()),
        linux_nr::GETTID => Some(task_time::sys_linux_gettid()),
        linux_nr::SET_TID_ADDRESS => Some(task_time::sys_linux_set_tid_address(arg1)),
        linux_nr::ARCH_PRCTL => Some(super::sys_linux_arch_prctl(arg1, arg2)),
        linux_nr::FUTEX => Some(super::sys_linux_futex(arg1, arg2, arg3)),
        linux_nr::TGKILL => Some(task_time::sys_linux_tgkill(arg1, arg2, arg3)),
        linux_nr::KILL => Some(task_time::sys_linux_kill(arg1, arg2)),
        linux_nr::CLOCK_GETTIME => Some(task_time::sys_linux_clock_gettime(arg1, arg2)),
        linux_nr::CLOCK_NANOSLEEP => {
            Some(task_time::sys_linux_clock_nanosleep(arg1, arg2, arg3, arg4))
        }
        linux_nr::BRK => Some(arg1),
        linux_nr::MMAP => Some(memory::sys_linux_mmap(arg1, arg2, arg3, arg4, arg5, arg6)),
        linux_nr::MPROTECT => Some(memory::sys_linux_mprotect(arg1, arg2, arg3)),
        linux_nr::MUNMAP => Some(memory::sys_linux_munmap(arg1, arg2)),
        #[cfg(feature = "linux_compat")]
        linux_nr::RT_SIGACTION => Some(crate::modules::linux_compat::sys_linux_rt_sigaction(
            arg1,
            crate::modules::linux_compat::UserPtr::new(arg2),
            crate::modules::linux_compat::UserPtr::new(arg3),
            arg4,
        )),
        #[cfg(not(feature = "linux_compat"))]
        linux_nr::RT_SIGACTION => Some(signal::sys_linux_rt_sigaction_shim(arg1, arg2, arg3, arg4)),
        #[cfg(feature = "linux_compat")]
        linux_nr::RT_SIGPROCMASK => Some(crate::modules::linux_compat::sys_linux_rt_sigprocmask(
            arg1,
            crate::modules::linux_compat::UserPtr::new(arg2),
            crate::modules::linux_compat::UserPtr::new(arg3),
            arg4,
        )),
        #[cfg(not(feature = "linux_compat"))]
        linux_nr::RT_SIGPROCMASK => Some(signal::sys_linux_rt_sigprocmask_shim(
            arg1, arg2, arg3, arg4,
        )),
        #[cfg(feature = "linux_compat")]
        linux_nr::IOCTL => Some(crate::modules::linux_compat::sys_linux_ioctl(
            crate::modules::linux_compat::Fd::from(arg1),
            arg2,
            arg3,
        )),
        #[cfg(not(feature = "linux_compat"))]
        linux_nr::IOCTL => Some(net::sys_linux_ioctl(arg1, arg2, arg3)),
        #[cfg(feature = "linux_compat")]
        linux_nr::SENDMSG => Some(crate::modules::linux_compat::sys_linux_sendmsg(
            crate::modules::linux_compat::Fd::from(arg1),
            crate::modules::linux_compat::UserPtr::new(arg2),
            arg3,
        )),
        #[cfg(not(feature = "linux_compat"))]
        linux_nr::SENDMSG => Some(net::sys_linux_sendmsg(arg1, arg2, arg3)),
        #[cfg(feature = "linux_compat")]
        linux_nr::RECVMSG => Some(crate::modules::linux_compat::sys_linux_recvmsg(
            crate::modules::linux_compat::Fd::from(arg1),
            crate::modules::linux_compat::UserPtr::new(arg2),
            arg3,
        )),
        #[cfg(not(feature = "linux_compat"))]
        linux_nr::RECVMSG => Some(net::sys_linux_recvmsg(arg1, arg2, arg3)),
        linux_nr::CLONE => Some(process::sys_linux_clone(arg1, arg2, arg3, arg4, arg5, arg6)),
        linux_nr::CLONE3 => Some(process::sys_linux_clone3(arg1, arg2)),
        linux_nr::FORK | linux_nr::VFORK => Some(process::sys_linux_fork()),
        linux_nr::EXECVE => Some(process::sys_linux_execve(arg1, arg2, arg3)),
        linux_nr::EXECVEAT => Some(process::sys_linux_execveat(arg1 as isize, arg2, arg3, arg4, arg5)),
        linux_nr::OPENAT => Some(fs::sys_linux_openat(arg1 as isize, arg2, arg3, arg4)),
        linux_nr::OPENAT2 => Some(fs::sys_linux_openat2(arg1 as isize, arg2, arg3, arg4)),
        linux_nr::MKDIRAT => Some(fs::sys_linux_mkdirat(arg1 as isize, arg2, arg3)),
        linux_nr::UNLINKAT => Some(fs::sys_linux_unlinkat(arg1 as isize, arg2, arg3)),
        linux_nr::LINKAT => Some(fs::sys_linux_linkat(
            arg1 as isize,
            arg2,
            arg3 as isize,
            arg4,
            arg5,
        )),
        linux_nr::SYMLINKAT => Some(fs::sys_linux_symlinkat(arg1, arg2 as isize, arg3)),
        linux_nr::RENAMEAT => Some(fs::sys_linux_renameat(
            arg1 as isize,
            arg2,
            arg3 as isize,
            arg4,
        )),
        linux_nr::RENAMEAT2 => Some(fs::sys_linux_renameat2(
            arg1 as isize,
            arg2,
            arg3 as isize,
            arg4,
            arg5,
        )),
        linux_nr::READLINKAT => Some(fs::sys_linux_readlinkat(arg1 as isize, arg2, arg3, arg4)),
        linux_nr::FSTAT => Some(fs::sys_linux_fstat(arg1, arg2)),
        linux_nr::NEWFSTATAT => Some(fs::sys_linux_newfstatat(arg1, arg2, arg3, arg4)),
        linux_nr::STATX => Some(fs::sys_linux_statx(arg1, arg2, arg3, arg4, arg5)),
        linux_nr::CHMOD => Some(fs::sys_linux_chmod(arg1, arg2)),
        linux_nr::FCHMOD => Some(fs::sys_linux_fchmod(arg1, arg2)),
        linux_nr::CHOWN => Some(fs::sys_linux_chown(arg1, arg2, arg3)),
        linux_nr::FCHOWN => Some(fs::sys_linux_fchown(arg1, arg2, arg3)),
        linux_nr::FCHOWNAT => Some(fs::sys_linux_fchownat(arg1, arg2, arg3, arg4, arg5)),
        linux_nr::FCHMODAT => Some(fs::sys_linux_fchmodat(arg1, arg2, arg3, arg4)),
        linux_nr::STATFS => Some(fs::sys_linux_statfs(arg1, arg2)),
        linux_nr::FSTATFS => Some(fs::sys_linux_fstatfs(arg1, arg2)),
        linux_nr::PIPE => Some(fd_process_identity::sys_linux_pipe(arg1, 0)),
        linux_nr::PIPE2 => Some(fd_process_identity::sys_linux_pipe(arg1, arg2)),
        linux_nr::DUP => Some(fd_process_identity::sys_linux_dup(arg1)),
        linux_nr::DUP2 => Some(fd_process_identity::sys_linux_dup2(arg1, arg2)),
        linux_nr::DUP3 => Some(fd_process_identity::sys_linux_dup3(arg1, arg2, arg3)),
        linux_nr::FCNTL => Some(fd_process_identity::sys_linux_fcntl(arg1, arg2, arg3)),
        linux_nr::GETDENTS64 => Some(fd_process_identity::sys_linux_getdents64(arg1, arg2, arg3)),
        linux_nr::GETCWD => Some(fd_process_identity::sys_linux_getcwd(arg1, arg2)),
        linux_nr::UNAME => Some(fd_process_identity::sys_linux_uname(arg1)),
        linux_nr::GETUID => Some(fd_process_identity::sys_linux_getuid()),
        linux_nr::GETGID => Some(fd_process_identity::sys_linux_getgid()),
        linux_nr::GETEUID => Some(fd_process_identity::sys_linux_geteuid()),
        linux_nr::GETEGID => Some(fd_process_identity::sys_linux_getegid()),
        linux_nr::GETPGRP => Some(fd_process_identity::sys_linux_getpgrp()),
        linux_nr::GETPGID => Some(fd_process_identity::sys_linux_getpgid(arg1)),
        linux_nr::SETPGID => Some(fd_process_identity::sys_linux_setpgid(arg1, arg2)),
        linux_nr::SETSID => Some(fd_process_identity::sys_linux_setsid()),
        linux_nr::GETRLIMIT => Some(super::linux_process::sys_linux_getrlimit(arg1, arg2)),
        linux_nr::SETRLIMIT => Some(super::linux_process::sys_linux_setrlimit(arg1, arg2)),
        linux_nr::PRLIMIT64 => Some(super::linux_process::sys_linux_prlimit64(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::WAIT4 => Some(super::linux_process::sys_linux_wait4(
            arg1 as isize,
            arg2,
            arg3,
            arg4,
        )),
        linux_nr::WAITID => Some(super::linux_process::sys_linux_waitid(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::ACCEPT4 => Some(net::sys_linux_accept(arg1, arg2, arg3, arg4 as i32)),
        linux_nr::MADVISE => Some(memory::sys_linux_madvise(arg1, arg2, arg3)),
        linux_nr::MLOCK => Some(memory::sys_linux_mlock(arg1, arg2)),
        linux_nr::MUNLOCK => Some(memory::sys_linux_munlock(arg1, arg2)),
        linux_nr::MLOCKALL => Some(memory::sys_linux_mlockall(arg1)),
        linux_nr::MUNLOCKALL => Some(memory::sys_linux_munlockall()),
        linux_nr::NANOSLEEP => Some(task_time::sys_linux_clock_nanosleep(0, 0, arg1, arg2)),
        linux_nr::EPOLL_CREATE1 => Some(net::sys_linux_epoll_create1(arg1)),
        linux_nr::EPOLL_CTL => Some(net::sys_linux_epoll_ctl(arg1, arg2, arg3, arg4)),
        linux_nr::EPOLL_PWAIT => Some(net::sys_linux_epoll_pwait(
            arg1, arg2, arg3, arg4, arg5, arg6,
        )),
        linux_nr::EPOLL_WAIT => Some(net::sys_linux_epoll_pwait(arg1, arg2, arg3, arg4, 0, 0)),
        linux_nr::EPOLL_CREATE => Some(net::sys_linux_epoll_create(arg1)),
        linux_nr::POLL => Some(super::linux_misc::sys_linux_poll(arg1, arg2, arg3)),
        linux_nr::PPOLL => Some(super::linux_misc::sys_linux_ppoll(
            arg1, arg2, arg3, arg4, arg5,
        )),
        linux_nr::SELECT => Some(super::linux_misc::sys_linux_select(
            arg1, arg2, arg3, arg4, arg5,
        )),
        linux_nr::PSELECT6 => Some(super::linux_misc::sys_linux_pselect6(
            arg1, arg2, arg3, arg4, arg5, arg6,
        )),
        linux_nr::EVENTFD => Some(super::linux_misc::sys_linux_eventfd(arg1, arg2)),
        linux_nr::EVENTFD2 => Some(super::linux_misc::sys_linux_eventfd2(arg1, arg2)),
        linux_nr::TIMERFD_CREATE => Some(super::linux_misc::sys_linux_timerfd_create(arg1, arg2)),
        linux_nr::TIMERFD_SETTIME => Some(super::linux_misc::sys_linux_timerfd_settime(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::TIMERFD_GETTIME => Some(super::linux_misc::sys_linux_timerfd_gettime(arg1, arg2)),
        linux_nr::SIGNALFD => Some(super::linux_misc::sys_linux_signalfd(arg1, arg2, arg3)),
        linux_nr::SIGNALFD4 => Some(super::linux_misc::sys_linux_signalfd4(arg1, arg2, arg3, arg4)),
        linux_nr::INOTIFY_INIT => Some(super::linux_misc::sys_linux_inotify_init()),
        linux_nr::INOTIFY_INIT1 => Some(super::linux_misc::sys_linux_inotify_init1(arg1)),
        linux_nr::INOTIFY_ADD_WATCH => {
            Some(super::linux_misc::sys_linux_inotify_add_watch(arg1, arg2, arg3))
        }
        linux_nr::INOTIFY_RM_WATCH => Some(super::linux_misc::sys_linux_inotify_rm_watch(arg1, arg2)),
        linux_nr::MEMFD_CREATE => Some(super::linux_misc::sys_linux_memfd_create(arg1, arg2)),
        linux_nr::FSYNC => Some(fs::sys_linux_fsync(arg1)),
        linux_nr::FDATASYNC => Some(fs::sys_linux_fdatasync(arg1)),
        linux_nr::SYNC => Some(fs::sys_linux_sync()),
        linux_nr::SYNCFS => Some(fs::sys_linux_syncfs(arg1)),
        linux_nr::FTRUNCATE => Some(fs::sys_linux_ftruncate(arg1, arg2)),
        linux_nr::FUTIMESAT => Some(fs::sys_linux_futimesat(arg1, arg2, arg3)),
        linux_nr::UTIMENSAT => Some(fs::sys_linux_utimensat(arg1, arg2, arg3, arg4)),
        linux_nr::SET_ROBUST_LIST => Some(task_time::sys_linux_set_robust_list(arg1, arg2)),
        linux_nr::GET_ROBUST_LIST => Some(task_time::sys_linux_get_robust_list(arg1, arg2, arg3)),
        linux_nr::RT_SIGRETURN => Some(signal::sys_linux_rt_sigreturn_shim(unsafe {
            &mut *frame_ptr
        })),
        linux_nr::SIGALTSTACK => Some(signal::sys_linux_sigaltstack_shim(arg1, arg2)),
        linux_nr::RT_SIGPENDING => Some(signal::sys_linux_rt_sigpending_shim(arg1, arg2)),
        linux_nr::GETTIMEOFDAY => Some(super::linux_misc::sys_linux_gettimeofday(arg1)),
        linux_nr::TIME => Some(super::linux_misc::sys_linux_time(arg1)),
        linux_nr::UMASK => Some(super::sys_linux_umask(arg1)),
        linux_nr::ACCESS => Some(fs::sys_linux_access(arg1, arg2)),
        linux_nr::FACCESSAT => Some(fs::sys_linux_faccessat(arg1 as isize, arg2, arg3, arg4)),
        linux_nr::FACCESSAT2 => Some(fs::sys_linux_faccessat2(arg1 as isize, arg2, arg3, arg4)),
        linux_nr::GETCPU => Some(super::linux_misc::sys_linux_getcpu(arg1, arg2)),
        linux_nr::PRCTL => Some(super::linux_misc::sys_linux_prctl(
            arg1, arg2, arg3, arg4, arg5,
        )),
        linux_nr::SCHED_GETSCHEDULER => Some(super::linux_misc::sys_linux_sched_getscheduler(arg1)),
        linux_nr::SCHED_SETSCHEDULER => Some(super::linux_misc::sys_linux_sched_setscheduler(
            arg1, arg2, arg3,
        )),
        linux_nr::SCHED_GETPARAM => Some(super::linux_misc::sys_linux_sched_getparam(arg1, arg2)),
        linux_nr::SCHED_SETPARAM => Some(super::linux_misc::sys_linux_sched_setparam(arg1, arg2)),
        linux_nr::SCHED_GETAFFINITY => Some(super::linux_misc::sys_linux_sched_getaffinity(
            arg1, arg2, arg3,
        )),
        linux_nr::SCHED_SETAFFINITY => Some(super::linux_misc::sys_linux_sched_setaffinity(
            arg1, arg2, arg3,
        )),
        linux_nr::SCHED_GET_PRIORITY_MAX => Some(task_time::sys_linux_sched_get_priority_max(arg1)),
        linux_nr::SCHED_GET_PRIORITY_MIN => Some(task_time::sys_linux_sched_get_priority_min(arg1)),
        linux_nr::UNSHARE => Some(process::sys_linux_unshare(arg1)),
        linux_nr::SETNS => Some(process::sys_linux_setns(arg1, arg2)),
        linux_nr::SYSINFO => Some(super::linux_misc::sys_linux_sysinfo(arg1)),
        linux_nr::MEMBARRIER => Some(super::linux_misc::sys_linux_membarrier(arg1, arg2, arg3)),
        linux_nr::GETRANDOM => Some(super::linux_misc::sys_linux_getrandom(arg1, arg2, arg3)),
        linux_nr::RSEQ => Some(super::linux_misc::sys_linux_rseq(arg1, arg2, arg3, arg4)),
        linux_nr::BPF => Some(super::linux_misc::sys_linux_bpf(arg1, arg2, arg3)),
        linux_nr::IO_URING_SETUP => Some(super::linux_misc::sys_linux_io_uring_setup(arg1, arg2)),
        linux_nr::IO_URING_ENTER => Some(super::linux_misc::sys_linux_io_uring_enter(
            arg1, arg2, arg3, arg4, arg5, arg6,
        )),
        linux_nr::IO_URING_REGISTER => Some(super::linux_misc::sys_linux_io_uring_register(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::LANDLOCK_CREATE_RULESET => Some(
            super::linux_misc::sys_linux_landlock_create_ruleset(arg1, arg2, arg3),
        ),
        linux_nr::LANDLOCK_ADD_RULE => Some(super::linux_misc::sys_linux_landlock_add_rule(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::LANDLOCK_RESTRICT_SELF => Some(
            super::linux_misc::sys_linux_landlock_restrict_self(arg1, arg2),
        ),
        linux_nr::FANOTIFY_INIT => Some(super::linux_misc::sys_linux_fanotify_init(arg1, arg2)),
        linux_nr::FANOTIFY_MARK => Some(super::linux_misc::sys_linux_fanotify_mark(
            arg1,
            arg2,
            arg3,
            arg4 as isize,
            arg5,
        )),
        linux_nr::FUTEX_WAITV => Some(super::linux_misc::sys_linux_futex_waitv(arg1, arg2, arg3, arg4)),
        linux_nr::CLOSE_RANGE => Some(fd_process_identity::sys_linux_close_range(arg1, arg2, arg3)),
        linux_nr::PIDFD_OPEN => Some(fd_process_identity::sys_linux_pidfd_open(arg1, arg2)),
        linux_nr::PIDFD_GETFD => Some(fd_process_identity::sys_linux_pidfd_getfd(arg1, arg2, arg3)),
        linux_nr::PIDFD_SEND_SIGNAL => Some(fd_process_identity::sys_linux_pidfd_send_signal(
            arg1, arg2, arg3, arg4,
        )),
        linux_nr::MREMAP => Some(memory::sys_linux_mremap(arg1, arg2, arg3, arg4, arg5)),
        linux_nr::EPOLL_PWAIT2 => Some(net::sys_linux_epoll_pwait2(
            arg1, arg2, arg3, arg4, arg5, arg6,
        )),
        _ => Some(linux_unknown_syscall_errno(syscall_id)),
    }
}

fn dispatch_socket_syscalls(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> Option<usize> {
    match syscall_id {
        linux_nr::SOCKET => Some(net::sys_linux_socket(arg1, arg2, arg3)),
        linux_nr::CONNECT => Some(net::sys_linux_connect(arg1, arg2, arg3)),
        linux_nr::BIND => Some(net::sys_linux_bind(arg1, arg2, arg3)),
        linux_nr::LISTEN => Some(net::sys_linux_listen(arg1, arg2)),
        linux_nr::ACCEPT => Some(net::sys_linux_accept(arg1, arg2, arg3, 0)),
        linux_nr::SENDTO => Some(net::sys_linux_sendto(
            arg1,
            arg2,
            arg3,
            arg4,
            arg5_to_zero(arg5),
            arg6,
        )),
        linux_nr::RECVFROM => Some(net::sys_linux_recvfrom(
            arg1,
            arg2,
            arg3,
            arg4,
            arg5_to_zero(arg5),
            arg6,
        )),
        linux_nr::SHUTDOWN => Some(net::sys_linux_shutdown(arg1, arg2)),
        linux_nr::GETSOCKNAME => Some(net::sys_linux_getsockname(arg1, arg2, arg3)),
        linux_nr::GETPEERNAME => Some(net::sys_linux_getpeername(arg1, arg2, arg3)),
        linux_nr::SETSOCKOPT => Some(net::sys_linux_setsockopt(
            arg1,
            arg2,
            arg3,
            arg4,
            arg5_to_zero(arg5),
        )),
        linux_nr::GETSOCKOPT => Some(net::sys_linux_getsockopt(
            arg1,
            arg2,
            arg3,
            arg4,
            arg5_to_zero(arg5),
        )),
        linux_nr::SOCKETPAIR => Some(net::sys_linux_socketpair(arg1, arg2, arg3, arg4)),
        _ => None,
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn unknown_syscall_policy_uses_eperm_for_privileged_numbers() {
        assert_eq!(
            linux_unknown_syscall_errno(linux_nr::REBOOT),
            linux_errno(crate::modules::posix_consts::errno::EPERM)
        );
        assert_eq!(
            linux_unknown_syscall_errno(linux_nr::KEXEC_LOAD),
            linux_errno(crate::modules::posix_consts::errno::EPERM)
        );
    }

    #[test_case]
    fn unknown_syscall_policy_keeps_enosys_for_truly_unknown_numbers() {
        let unknown_nr = linux_nr::PIDFD_GETFD + 10_000;
        assert_eq!(
            linux_unknown_syscall_errno(unknown_nr),
            linux_errno(crate::modules::posix_consts::errno::ENOSYS)
        );
    }
}
