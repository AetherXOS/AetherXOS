use super::*;
use super::support::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigprocmask_shim(
    how: usize,
    set: usize,
    oldset: usize,
    sigsetsize: usize,
) -> usize {
    let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
    if let Err(err) = validate_linux_sigset_size(sigsetsize) {
        return err;
    }
    let how = match decode_sigprocmask_how(how) {
        Ok(v) => v,
        Err(err) => return err,
    };

    let task_arc = match current_task_arc_for_signal_shim() {
        Ok(task_arc) => task_arc,
        Err(err) => return err,
    };

    let mut task = task_arc.lock();
    let old_mask = task.signal_mask;

    if should_write_signal_set(oldset) {
        if let Err(err) = write_signal_set(oldset, old_mask) {
            return err;
        }
    }

    if set != 0 {
        let raw_mask = match read_signal_set(set) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let new_mask = sanitize_linux_sigmask(raw_mask);

        task.signal_mask = match how {
            crate::modules::posix_consts::signal::SIG_BLOCK => old_mask | new_mask,
            crate::modules::posix_consts::signal::SIG_UNBLOCK => old_mask & !new_mask,
            crate::modules::posix_consts::signal::SIG_SETMASK => new_mask,
            _ => unreachable!("validated above"),
        };
    }

    0
}
