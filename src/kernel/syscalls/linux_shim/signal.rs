mod support;

use super::*;
use support::*;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_rt_sigaction_shim(
    signum: usize,
    act: usize,
    oldact: usize,
    sigsetsize: usize,
) -> usize {
    if let Err(err) = validate_sigaction_args(signum, sigsetsize) {
        return err;
    }
    #[cfg(feature = "posix_signal")]
    {
        use crate::modules::posix::signal;

        let sig = signum as i32;
        if sig == linux::SIGKILL || sig == linux::SIGSTOP {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }

        if oldact != 0 {
            let pid = signal::current_pid_pub();
            let actions = signal::SIGNAL_ACTIONS.lock();
            let previous = actions
                .get(&(pid, sig))
                .copied()
                .map(|action| LinuxKSigActionCompat {
                    sa_handler: action.handler.map(|h| h as usize as u64).unwrap_or(0),
                    sa_flags: action.flags as u64,
                    sa_restorer: action.restorer,
                    sa_mask: action.mask,
                })
                .unwrap_or_default();
            if let Err(err) = write_sigaction(oldact, &previous) {
                return err;
            }
        }

        if act != 0 {
            let action = match read_sigaction(act) {
                Ok(v) => v,
                Err(err) => return err,
            };
            let handler = if action.sa_handler == 0 {
                None
            } else if action.sa_handler == 1 {
                Some(signal::sig_ign as signal::SignalHandler)
            } else {
                unsafe {
                    Some(core::mem::transmute::<usize, signal::SignalHandler>(
                        action.sa_handler as usize,
                    ))
                }
            };

            if signal::signal_action(
                sig,
                handler,
                action.sa_mask,
                action.sa_flags as u32,
                action.sa_restorer,
            )
            .is_err()
            {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
        }

        0
    }

    #[cfg(not(feature = "posix_signal"))]
    {
        let _ = act;
        if oldact != 0 {
            if let Err(err) = write_zeroed_sigaction(oldact) {
                return err;
            }
        }
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_rt_sigprocmask_shim(
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
        let new_mask = match read_signal_set(set) {
            Ok(v) => v,
            Err(err) => return err,
        };

        task.signal_mask = match how {
            crate::modules::posix_consts::signal::SIG_BLOCK => old_mask | new_mask,
            crate::modules::posix_consts::signal::SIG_UNBLOCK => old_mask & !new_mask,
            crate::modules::posix_consts::signal::SIG_SETMASK => new_mask,
            _ => unreachable!("validated above"),
        };
    }

    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sigaltstack_shim(ss_ptr: usize, old_ss_ptr: usize) -> usize {
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

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_rt_sigpending_shim(set_ptr: usize, sigsetsize: usize) -> usize {
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

#[cfg(all(not(feature = "linux_compat"), target_arch = "x86_64"))]
pub(super) fn sys_linux_rt_sigreturn_shim(
    frame: &mut crate::kernel::syscalls::SyscallFrame,
) -> usize {
    let uctx = match read_ucontext(frame.rsi as usize) {
        Ok(v) => v,
        Err(_) => match read_ucontext((frame.rsi as usize).saturating_add(8)) {
            Ok(v) => v,
            Err(err) => return err,
        },
    };

    let m = &uctx.mcontext;
    frame.r15 = m.r15;
    frame.r14 = m.r14;
    frame.r13 = m.r13;
    frame.r12 = m.r12;
    frame.rbp = m.rbp;
    frame.rbx = m.rbx;
    frame.rflags = m.eflags;
    frame.rax = m.rax;
    frame.rdx = m.rdx;
    frame.rsi = m.rsi;
    frame.rdi = m.rdi;
    frame.rip = m.rip;

    #[cfg(feature = "posix_signal")]
    {
        let _ = crate::modules::posix::signal::sigprocmask(
            crate::modules::posix::signal::SigmaskHow::SetMask,
            Some(uctx.sigmask),
        );
    }

    frame.rax as usize
}

#[cfg(all(not(feature = "linux_compat"), not(target_arch = "x86_64")))]
pub(super) fn sys_linux_rt_sigreturn_shim(
    _frame: &mut crate::kernel::syscalls::SyscallFrame,
) -> usize {
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

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
    fn sigaltstack_without_task_context_returns_esrch() {
        assert_eq!(
            sys_linux_sigaltstack_shim(0, 0),
            linux_errno(crate::modules::posix_consts::errno::ESRCH)
        );
    }
}
