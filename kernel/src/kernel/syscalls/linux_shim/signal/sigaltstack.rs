use super::*;
use super::support::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_sigaltstack_shim(ss_ptr: usize, old_ss_ptr: usize) -> usize {
    let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
    let task_arc = match current_task_arc_for_signal_shim() {
        Ok(task_arc) => task_arc,
        Err(err) => return err,
    };
    let mut task = task_arc.lock();

    if old_ss_ptr != 0 {
        let old = match task.signal_stack {
            Some(ss) => LinuxSigaltstackCompat {
                ss_sp: ss.ss_sp,
                ss_flags: ss.ss_flags,
                ss_size: ss.ss_size,
            },
            None => LinuxSigaltstackCompat {
                ss_sp: 0,
                ss_flags: linux::SS_DISABLE as i32,
                ss_size: 0,
            },
        };
        if let Err(err) = write_sigaltstack(old_ss_ptr, &old) {
            return err;
        }
    }

    if ss_ptr != 0 {
        let new_ss = match read_sigaltstack(ss_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if let Err(err) = validate_sigaltstack_flags_and_size(&new_ss) {
            return err;
        }

        if (new_ss.ss_flags & (linux::SS_DISABLE as i32)) != 0 {
            task.signal_stack = None;
        } else {
            task.signal_stack = Some(crate::interfaces::task::SignalStack {
                ss_sp: new_ss.ss_sp,
                ss_flags: new_ss.ss_flags,
                ss_size: new_ss.ss_size,
            });
        }
    }

    0
}
