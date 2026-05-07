use super::*;
use super::support::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigsuspend_shim(unmask_ptr: usize, sigsetsize: usize) -> usize {
    if let Err(err) = validate_linux_sigset_size(sigsetsize) {
        return err;
    }

    let temporary_mask = if should_write_signal_set(unmask_ptr) {
        match read_signal_set(unmask_ptr) {
            Ok(mask) => sanitize_linux_sigmask(mask),
            Err(err) => return err,
        }
    } else {
        0
    };

    #[cfg(feature = "posix_signal")]
    {
        use crate::modules::posix::signal;

        match signal::sigsuspend(temporary_mask) {
            Ok(_) => linux_errno(crate::modules::posix_consts::errno::EINTR),
            Err(err) => {
                let errno = err.code();
                if errno == crate::modules::posix_consts::errno::ETIMEDOUT {
                    linux_errno(crate::modules::posix_consts::errno::EINTR)
                } else {
                    linux_errno(errno)
                }
            }
        }
    }

    #[cfg(not(feature = "posix_signal"))]
    {
        let task_arc = match current_task_arc_for_signal_shim() {
            Ok(task_arc) => task_arc,
            Err(err) => return err,
        };
        let mut task = task_arc.lock();
        let old_mask = task.signal_mask;
        task.signal_mask = temporary_mask;
        task.signal_mask = old_mask;
        linux_errno(crate::modules::posix_consts::errno::EINTR)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_pause_shim() -> usize {
    #[cfg(feature = "posix_signal")]
    {
        match crate::modules::posix::signal::pause() {
            Ok(_) => linux_errno(crate::modules::posix_consts::errno::EINTR),
            Err(err) => {
                let errno = err.code();
                if errno == crate::modules::posix_consts::errno::ETIMEDOUT {
                    linux_errno(crate::modules::posix_consts::errno::EINTR)
                } else {
                    linux_errno(errno)
                }
            }
        }
    }

    #[cfg(not(feature = "posix_signal"))]
    {
        if let Err(err) = current_task_arc_for_signal_shim() {
            return err;
        }
        linux_errno(crate::modules::posix_consts::errno::EINTR)
    }
}
