pub mod bpf;
pub mod eventfd;
pub mod fanotify;
pub mod futex;
pub mod inotify;
pub mod io_uring;
pub mod landlock;
pub mod memfd;
pub mod misc;
pub mod poll_select;
pub mod proc_ctl;
pub mod runtime_info;
pub mod signalfd;
pub mod state;
pub mod timerfd;
pub mod types;
pub mod utils;
#[allow(unused_imports)]
pub use types::*;

#[cfg(all(test, not(feature = "linux_compat")))]
mod runtime_stress_tests;

#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_errno;

// Re-export public syscall handlers
#[cfg(not(feature = "linux_compat"))]
pub use self::{
    bpf::sys_linux_bpf,
    eventfd::{sys_linux_eventfd, sys_linux_eventfd2},
    fanotify::{sys_linux_fanotify_init, sys_linux_fanotify_mark},
    futex::sys_linux_futex_waitv,
    inotify::{sys_linux_inotify_add_watch, sys_linux_inotify_init, sys_linux_inotify_init1, sys_linux_inotify_rm_watch},
    io_uring::{sys_linux_io_uring_enter, sys_linux_io_uring_register, sys_linux_io_uring_setup},
    landlock::{sys_linux_landlock_add_rule, sys_linux_landlock_create_ruleset, sys_linux_landlock_restrict_self},
    memfd::sys_linux_memfd_create,
    misc::{sys_linux_membarrier, sys_linux_rseq},
    poll_select::{sys_linux_poll, sys_linux_ppoll, sys_linux_pselect6, sys_linux_select},
    proc_ctl::{sys_linux_prctl, sys_linux_sched_getaffinity, sys_linux_sched_getparam, sys_linux_sched_getscheduler, sys_linux_sched_setaffinity, sys_linux_sched_setparam, sys_linux_sched_setscheduler},
    runtime_info::{sys_linux_getcpu, sys_linux_getrandom, sys_linux_gettimeofday, sys_linux_sysinfo, sys_linux_time},
    signalfd::{sys_linux_signalfd, sys_linux_signalfd4},
    timerfd::{sys_linux_timerfd_create, sys_linux_timerfd_gettime, sys_linux_timerfd_settime},
};

// Internal utility exports
#[cfg(not(feature = "linux_compat"))]
pub(crate) use self::{
    proc_ctl::{
        no_new_privs_for_tid as linux_prctl_no_new_privs_for_tid,
        seccomp_mode_for_tid as linux_prctl_seccomp_mode_for_tid,
        validate_syscall_for_current_task,
    },
    timerfd::poll_revents as timerfd_poll_revents,
};

#[cfg(all(test, not(feature = "linux_compat")))]
pub(crate) use self::proc_ctl::set_prctl_state_for_tid_for_test as linux_set_prctl_state_for_tid_for_test;

#[cfg(all(test, not(feature = "linux_compat")))]
mod modern_syscall_policy_tests {
    use super::*;

    #[test_case]
    fn io_uring_setup_rejects_zero_entries() {
        let params = [0u8; 16];
        assert_eq!(
            sys_linux_io_uring_setup(0, params.as_ptr() as usize),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn bpf_map_create_rejects_null_attr() {
        assert_eq!(
            sys_linux_bpf(0, 0, 16),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn landlock_ruleset_rejects_nonzero_flags() {
        let attr = [0u8; 16];
        assert_eq!(
            sys_linux_landlock_create_ruleset(attr.as_ptr() as usize, attr.len(), 1),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn fanotify_init_allocates_descriptor() {
        let fd = sys_linux_fanotify_init(0, 0);
        assert!(fd > 0, "fanotify_init should return a valid descriptor token");
    }

    #[test_case]
    fn fanotify_mark_rejects_invalid_fd() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_fanotify_mark(0xFFFF, 0x1, 1, -1, path.as_ptr() as usize),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }

    #[test_case]
    fn futex_waitv_rejects_invalid_flags() {
        let waiter = LinuxFutexWaitVCompat::default();
        assert_eq!(
            sys_linux_futex_waitv((&waiter as *const LinuxFutexWaitVCompat) as usize, 1, 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
