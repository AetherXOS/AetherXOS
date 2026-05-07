#[cfg(not(feature = "linux_compat"))]
use super::storage::{linux_fd_set_descriptor_flags, LINUX_FD_CLOEXEC, LINUX_PIDFD_MAP};
#[cfg(not(feature = "linux_compat"))]
use super::utils::{linux_current_tid, linux_pidfd_entry_for_caller, linux_pidfd_getfd_access_allowed, linux_task_exists};
#[cfg(not(feature = "linux_compat"))]
use super::storage::LinuxPidFdEntry;
#[cfg(not(feature = "linux_compat"))]
use super::duplication::sys_linux_dup;
#[cfg(not(feature = "linux_compat"))]
use super::super::super::super::linux_errno;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_close_range(first: usize, last: usize, flags: usize) -> usize {
    const CLOSE_RANGE_UNSHARE: usize = 1 << 1;
    const CLOSE_RANGE_CLOEXEC: usize = 1 << 2;

    if first > last {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !(CLOSE_RANGE_UNSHARE | CLOSE_RANGE_CLOEXEC)) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let cap_last = core::cmp::min(last, first.saturating_add(8192));
    for fd in first..=cap_last {
        if (flags & CLOSE_RANGE_CLOEXEC) != 0 {
            linux_fd_set_descriptor_flags(fd as u32, LINUX_FD_CLOEXEC);
        } else {
            let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd);
        }
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_open(pid: usize, flags: usize) -> usize {
    if pid == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !crate::kernel::syscalls::syscalls_consts::linux::PIDFD_NONBLOCK) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if !linux_task_exists(pid) {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    }

    let caller = linux_current_tid();
    if caller != pid && caller != 1 {
        return linux_errno(crate::modules::posix_consts::errno::EPERM);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let path = alloc::format!("/.pidfd-{}", pid);
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => {
                LINUX_PIDFD_MAP.lock().insert(
                    fd,
                    LinuxPidFdEntry {
                        target_pid: pid,
                        owner_tid: caller,
                    },
                );
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let fd = (SYNTH_PIDFD_BASE as u32).saturating_add(
            NEXT_SYNTH_PIDFD.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
        );
        LINUX_PIDFD_MAP.lock().insert(
            fd,
            LinuxPidFdEntry {
                target_pid: pid,
                owner_tid: caller,
            },
        );
        let _ = flags;
        fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_send_signal(
    pidfd: usize,
    sig: usize,
    _info_ptr: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if sig > 64 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let entry = match linux_pidfd_entry_for_caller(pidfd) {
        Ok(v) => v,
        Err(err) => return err,
    };

    if sig == 0 {
        return 0;
    }
    crate::kernel::syscalls::linux_shim::task_time::sys_linux_kill(entry.target_pid, sig)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_getfd(
    pidfd: usize,
    targetfd: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let entry = match linux_pidfd_entry_for_caller(pidfd) {
        Ok(v) => v,
        Err(err) => return err,
    };
    let caller_tid = linux_current_tid();
    if !linux_pidfd_getfd_access_allowed(caller_tid, entry.target_pid) {
        return linux_errno(crate::modules::posix_consts::errno::EPERM);
    }

    sys_linux_dup(targetfd)
}
