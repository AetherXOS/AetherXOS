use super::*;
use super::support::*;

#[cfg(not(feature = "linux_compat"))]
#[cfg(feature = "posix_signal")]
fn map_signal_wait_errno_for_sigwait(errno: i32) -> usize {
    if errno == crate::modules::posix_consts::errno::ETIMEDOUT {
        linux_errno(crate::modules::posix_consts::errno::EINTR)
    } else {
        linux_errno(errno)
    }
}

#[cfg(not(feature = "linux_compat"))]
#[cfg(feature = "posix_signal")]
fn map_signal_wait_errno_for_sigtimed(errno: i32) -> usize {
    if errno == crate::modules::posix_consts::errno::ETIMEDOUT {
        linux_errno(crate::modules::posix_consts::errno::EAGAIN)
    } else {
        linux_errno(errno)
    }
}

#[cfg(not(feature = "linux_compat"))]
fn decode_required_wait_mask(set_ptr: usize, sigsetsize: usize) -> Result<u64, usize> {
    validate_linux_sigset_size(sigsetsize)?;
    if !should_write_signal_set(set_ptr) {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let wait_mask = sanitize_linux_sigmask(read_signal_set(set_ptr)?);
    if wait_mask == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    Ok(wait_mask)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_signal"))]
fn finish_wait_signum_with_siginfo(siginfo_ptr: usize, signum: i32) -> usize {
    use crate::modules::posix::signal;

    match write_signal_wait_siginfo(siginfo_ptr, signum, signal::current_pid_pub()) {
        Ok(()) => signum as usize,
        Err(err) => err,
    }
}

#[cfg(all(not(feature = "linux_compat"), not(feature = "posix_signal")))]
fn finish_wait_signum_with_siginfo(siginfo_ptr: usize, signum: i32) -> usize {
    match write_signal_wait_siginfo(siginfo_ptr, signum, 0) {
        Ok(()) => signum as usize,
        Err(err) => err,
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigwaitinfo_shim(
    set_ptr: usize,
    siginfo_ptr: usize,
    sigsetsize: usize,
) -> usize {
    let wait_mask = match decode_required_wait_mask(set_ptr, sigsetsize) {
        Ok(mask) => mask,
        Err(err) => return err,
    };

    #[cfg(feature = "posix_signal")]
    {
        use crate::modules::posix::signal;

        match signal::sigwaitinfo(wait_mask) {
            Ok(signum) => finish_wait_signum_with_siginfo(siginfo_ptr, signum),
            Err(err) => map_signal_wait_errno_for_sigwait(err.code()),
        }
    }

    #[cfg(not(feature = "posix_signal"))]
    {
        let task_arc = match current_task_arc_for_signal_shim() {
            Ok(task_arc) => task_arc,
            Err(err) => return err,
        };
        let mut task = task_arc.lock();
        let available = task.pending_signals & wait_mask;
        let Some(signum) = first_signal_from_mask(available) else {
            return linux_errno(crate::modules::posix_consts::errno::EINTR);
        };

        let bit = 1u64 << ((signum as u64).saturating_sub(1));
        task.pending_signals &= !bit;
        finish_wait_signum_with_siginfo(siginfo_ptr, signum)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigtimedwait_shim(
    set_ptr: usize,
    siginfo_ptr: usize,
    timeout_ptr: usize,
    sigsetsize: usize,
) -> usize {
    let wait_mask = match decode_required_wait_mask(set_ptr, sigsetsize) {
        Ok(mask) => mask,
        Err(err) => return err,
    };

    let timeout_budget = match read_signal_wait_timeout_spin_budget(timeout_ptr) {
        Ok(v) => v,
        Err(err) => return err,
    };

    if timeout_budget.is_none() {
        return sys_linux_rt_sigwaitinfo_shim(set_ptr, siginfo_ptr, sigsetsize);
    }

    #[cfg(feature = "posix_signal")]
    {
        use crate::modules::posix::signal;

        match signal::sigtimedwait(wait_mask, timeout_budget.unwrap_or(0)) {
            Ok(Some(signum)) => finish_wait_signum_with_siginfo(siginfo_ptr, signum),
            Ok(None) => linux_errno(crate::modules::posix_consts::errno::EAGAIN),
            Err(err) => map_signal_wait_errno_for_sigtimed(err.code()),
        }
    }

    #[cfg(not(feature = "posix_signal"))]
    {
        let task_arc = match current_task_arc_for_signal_shim() {
            Ok(task_arc) => task_arc,
            Err(err) => return err,
        };
        let mut task = task_arc.lock();
        let available = task.pending_signals & wait_mask;
        let Some(signum) = first_signal_from_mask(available) else {
            return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
        };

        let bit = 1u64 << ((signum as u64).saturating_sub(1));
        task.pending_signals &= !bit;
        finish_wait_signum_with_siginfo(siginfo_ptr, signum)
    }
}
