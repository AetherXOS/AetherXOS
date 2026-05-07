use crate::kernel::syscalls::linux_errno;
use super::types::*;
use super::state::*;
use super::utils::*;

#[inline]
pub fn is_expired(state: &TimerfdRuntimeState, now_ns: u128) -> bool {
    let Some(initial_ns) = timespec_to_ns(state.spec.it_value) else {
        return false;
    };
    if initial_ns == 0 {
        return false;
    }
    now_ns.saturating_sub(state.armed_at_ns) >= initial_ns
}

#[inline]
pub fn current_spec(state: &TimerfdRuntimeState, now_ns: u128) -> LinuxItimerspecCompat {
    let mut out = state.spec;
    let Some(initial_ns) = timespec_to_ns(state.spec.it_value) else {
        out.it_value = LinuxTimespecCompat::default();
        return out;
    };
    if initial_ns == 0 {
        out.it_value = LinuxTimespecCompat::default();
        return out;
    }

    let elapsed_ns = now_ns.saturating_sub(state.armed_at_ns);
    let interval_ns = timespec_to_ns(state.spec.it_interval).unwrap_or(0);

    let remaining_ns = if elapsed_ns < initial_ns {
        initial_ns.saturating_sub(elapsed_ns)
    } else if interval_ns == 0 {
        0
    } else {
        let passed_after_first = elapsed_ns.saturating_sub(initial_ns);
        let rem = passed_after_first % interval_ns;
        if rem == 0 { interval_ns } else { interval_ns.saturating_sub(rem) }
    };

    out.it_value = ns_to_timespec(remaining_ns);
    out
}

pub fn poll_revents(fd: u32, requested_events: u16) -> u16 {
    let wants_read = (requested_events & crate::modules::posix_consts::net::POLLIN) != 0;
    if !wants_read {
        return 0;
    }
    let now_ns = monotonic_now_ns();
    let guard = TIMERFD_STATE_BY_FD.lock();
    let Some(state) = guard.get(&fd) else {
        return 0;
    };
    if is_expired(state, now_ns) {
        crate::modules::posix_consts::net::POLLIN
    } else {
        0
    }
}

pub fn sys_linux_timerfd_create(clockid: usize, flags: usize) -> usize {
    let allowed_flags = 0x1usize | 0x0008_0000usize | 0x0000_0800usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if clockid > crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", "timerfd", true) {
            Ok(fd) => {
                if (flags & 0x0000_0800usize) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    );
                }
                TIMERFD_STATE_BY_FD
                    .lock()
                    .insert(
                        fd,
                        TimerfdRuntimeState {
                            spec: LinuxItimerspecCompat::default(),
                            armed_at_ns: monotonic_now_ns(),
                        },
                    );
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_TIMERFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (TIMERFD_FD_BASE as u32).saturating_add(id);
        TIMERFD_STATE_BY_FD
            .lock()
            .insert(
                fd,
                TimerfdRuntimeState {
                    spec: LinuxItimerspecCompat::default(),
                    armed_at_ns: monotonic_now_ns(),
                },
            );
        fd as usize
    }
}

pub fn sys_linux_timerfd_settime(
    fd: usize,
    flags: usize,
    new_value_ptr: usize,
    old_value_ptr: usize,
) -> usize {
    if (flags & !0x1usize) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if new_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    let new_spec = match read_itimerspec_from_user(new_value_ptr) {
        Ok(v) => v,
        Err(err) => return err,
    };
    if !validate_timespec_compat(new_spec.it_interval) || !validate_timespec_compat(new_spec.it_value)
    {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let mut state = TIMERFD_STATE_BY_FD.lock();
    let Some(slot) = state.get_mut(&(fd as u32)) else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };

    if old_value_ptr != 0 {
        let rc = write_itimerspec_to_user(old_value_ptr, current_spec(slot, monotonic_now_ns()));
        if rc != 0 {
            return rc;
        }
    }

    slot.spec = new_spec;
    slot.armed_at_ns = monotonic_now_ns();
    0
}

pub fn sys_linux_timerfd_gettime(fd: usize, curr_value_ptr: usize) -> usize {
    if curr_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let state = TIMERFD_STATE_BY_FD.lock();
    let Some(spec) = state.get(&(fd as u32)).copied() else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };
    write_itimerspec_to_user(curr_value_ptr, current_spec(&spec, monotonic_now_ns()))
}
