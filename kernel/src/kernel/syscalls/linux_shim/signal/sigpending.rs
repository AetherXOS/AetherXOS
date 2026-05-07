use super::*;
use super::support::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigpending_shim(set_ptr: usize, sigsetsize: usize) -> usize {
    let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
    if let Err(err) = validate_linux_sigset_size(sigsetsize) {
        return err;
    }
    if !should_write_signal_set(set_ptr) {
        return 0;
    }

    let pending: u64 = {
        #[cfg(feature = "posix_signal")]
        {
            crate::modules::posix::signal::sigpending()
        }
        #[cfg(not(feature = "posix_signal"))]
        {
            match current_task_arc_for_signal_shim() {
                Ok(task_arc) => task_arc.lock().pending_signals,
                Err(err) => return err,
            }
        }
    };

    write_signal_set(set_ptr, pending).map_or_else(|err| err, |_| 0)
}
