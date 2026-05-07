use super::*;
use super::support::*;

#[test_case]
fn sigpending_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigpending_shim(0, linux::SIGSET_SIZE + 1),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[cfg(feature = "posix_signal")]
#[test_case]
fn sigpending_invalid_ptr_returns_efault() {
    assert_eq!(
        sys_linux_rt_sigpending_shim(1, linux::SIGSET_SIZE),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn sigaction_oldact_invalid_ptr_returns_efault() {
    assert_eq!(
        sys_linux_rt_sigaction_shim(1, 0, 1, 8),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn sigaction_rejects_invalid_signal_number() {
    assert_eq!(
        sys_linux_rt_sigaction_shim(0, 0, 0, 8),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
    assert_eq!(
        sys_linux_rt_sigaction_shim(65, 0, 0, 8),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigaction_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigaction_shim(1, 0, 0, 4),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigpending_zero_pointer_is_allowed() {
    assert_eq!(sys_linux_rt_sigpending_shim(0, linux::SIGSET_SIZE), 0);
}

#[test_case]
fn sigaction_zeroes_oldact_buffer() {
    let mut oldact = [0xAAu8; 32];
    assert_eq!(
        sys_linux_rt_sigaction_shim(1, 0, oldact.as_mut_ptr() as usize, 8),
        0
    );
    assert!(oldact.iter().all(|byte| *byte == 0));
}

#[test_case]
fn sigprocmask_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigprocmask_shim(0, 0, 0, 4),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigprocmask_without_task_context_returns_esrch() {
    assert_eq!(
        sys_linux_rt_sigprocmask_shim(
            crate::modules::posix_consts::signal::SIG_SETMASK as usize,
            0,
            0,
            core::mem::size_of::<u64>(),
        ),
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    );
}

#[test_case]
fn sigprocmask_rejects_invalid_how_before_task_lookup() {
    assert_eq!(
        sys_linux_rt_sigprocmask_shim(usize::MAX, 0, 0, core::mem::size_of::<u64>()),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigpending_zero_pointer_with_valid_sigset_size_is_a_noop() {
    assert_eq!(
        sys_linux_rt_sigpending_shim(0, core::mem::size_of::<u64>()),
        0
    );
}

#[test_case]
fn sigsuspend_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigsuspend_shim(0, core::mem::size_of::<u32>()),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigwaitinfo_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigwaitinfo_shim(0, 0, core::mem::size_of::<u32>()),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigwaitinfo_rejects_null_set_pointer() {
    assert_eq!(
        sys_linux_rt_sigwaitinfo_shim(0, 0, core::mem::size_of::<u64>()),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn sigtimedwait_rejects_invalid_sigset_size() {
    assert_eq!(
        sys_linux_rt_sigtimedwait_shim(0, 0, 0, core::mem::size_of::<u32>()),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn sigtimedwait_rejects_invalid_timeout_timespec() {
    let wait_mask = 1u64;
    let invalid_timeout = LinuxTimespecCompat {
        tv_sec: 0,
        tv_nsec: 1_000_000_000,
    };
    assert_eq!(
        sys_linux_rt_sigtimedwait_shim(
            (&wait_mask as *const u64) as usize,
            0,
            (&invalid_timeout as *const LinuxTimespecCompat) as usize,
            core::mem::size_of::<u64>(),
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[cfg(not(feature = "posix_signal"))]
#[test_case]
fn pause_without_task_context_returns_esrch() {
    assert_eq!(
        sys_linux_pause_shim(),
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    );
}

#[cfg(not(feature = "posix_signal"))]
#[test_case]
fn sigsuspend_without_task_context_returns_esrch() {
    assert_eq!(
        sys_linux_rt_sigsuspend_shim(0, core::mem::size_of::<u64>()),
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    );
}

#[test_case]
fn sigaltstack_without_task_context_returns_esrch() {
    assert_eq!(
        sys_linux_sigaltstack_shim(0, 0),
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    );
}
