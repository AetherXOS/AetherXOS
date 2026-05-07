use super::support::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_rt_sigaction_shim(
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
                    sa_mask: sanitize_linux_sigmask(action.mask),
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
                sanitize_linux_sigmask(action.sa_mask),
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
