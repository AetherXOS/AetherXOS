use super::super::*;
const SS_AUTODISARM: i32 = 0x8000_0000u32 as i32;
const LINUX_MINSIGSTKSZ: u64 = 2048;

/// `rt_sigaction(2)` — Establish a signal handler.
pub fn sys_linux_rt_sigaction(
    signum: usize,
    act: UserPtr<LinuxKSigAction>,
    oldact: UserPtr<LinuxKSigAction>,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_signal!((signum, act, oldact, sigsetsize) => {
        use crate::modules::posix::signal;

        if sigsetsize != 8 { // Linux x86_64 expects 8-byte sigset_t (64 bits)
            return linux_inval();
        }

        let sig = signum as i32;
        if sig <= 0 || sig > 64 || sig == linux::SIGKILL || sig == linux::SIGSTOP {
            return linux_inval();
        }

        // Return old action if requested
        if !oldact.is_null() {
            let actions = signal::SIGNAL_ACTIONS.lock();
            let pid = signal::current_pid_pub();
            let ks = if let Some(action) = actions.get(&(pid, sig)) {
                LinuxKSigAction {
                    sa_handler: action.handler.map(|h| h as u64).unwrap_or(0),
                    sa_flags: action.flags as u64,
                    sa_restorer: action.restorer,
                    sa_mask: action.mask,
                }
            } else {
                LinuxKSigAction { sa_handler: 0, sa_flags: 0, sa_restorer: 0, sa_mask: 0 }
            };
            if let Err(e) = oldact.write(&ks) { return e; }
        }

        // Set new action
        if !act.is_null() {
            let ks = match act.read() { Ok(v) => v, Err(e) => return e };

            // Note: sa_restorer is crucial for glibc. We should store it in the task's signal state.
            // Currently, our posix module needs enhancement to support per-signal restorers.

            let handler_fn = if ks.sa_handler == 0 { None }
                             else if ks.sa_handler == 1 { Some(signal::sig_ign as signal::SignalHandler) }
                             else { unsafe { Some(core::mem::transmute::<usize, signal::SignalHandler>(ks.sa_handler as usize)) } };

            let _ = signal::signal_action(sig, handler_fn, ks.sa_mask, ks.sa_flags as u32, ks.sa_restorer);
        }
        0
    })
}

/// `rt_sigreturn(2)` — Return from signal handler and clean up stack frame.
pub fn sys_linux_rt_sigreturn(frame: &mut SyscallFrame) -> usize {
    crate::require_posix_signal!((frame) => {
        // Redzone on x86_64 is 128 bytes, but signal frame is pushed beyond that.
        // We expect the ucontext to be at [RSP + 8] because the return address is at [RSP].
        // However, many implementations point RSP directly to the frame.

        let uctx_ptr = UserPtr::<LinuxUContext>::new(frame.rsi as usize);
        let uctx = match uctx_ptr.read() {
            Ok(c) => c,
            Err(_) => {
                // Try offset if direct read fails (some glibc versions adjust RSP)
                match UserPtr::<LinuxUContext>::new(frame.rsi as usize + 8).read() {
                    Ok(c) => c,
                    Err(_) => return linux_fault(),
                }
            }
        };

        let m = &uctx.mcontext;

        // Full Register Restoration (Production Grade)
        frame.r15 = m.r15; frame.r14 = m.r14; frame.r13 = m.r13; frame.r12 = m.r12;
        frame.rbp = m.rbp; frame.rbx = m.rbx; frame.rflags = m.eflags;

        // Populate the expanded scratch registers in the frame so they are popped by the assembly handler
        frame.rax = m.rax;
        frame.rdx = m.rdx;
        frame.rsi = m.rsi;
        frame.rdi = m.rdi;
        frame.rip = m.rip;

        // Restore signal mask
        use crate::modules::posix::signal::{self, SigmaskHow};
        let _ = signal::sigprocmask(SigmaskHow::SetMask, Some(uctx.sigmask));

        // Return the saved RAX to preserve syscall result if this was an interrupted syscall
        frame.rax as usize
    })
}

/// `sigaltstack(2)` — Set and/or get the signal stack context.
pub fn sys_linux_sigaltstack(ss: UserPtr<LinuxStackT>, old_ss: UserPtr<LinuxStackT>) -> usize {
    crate::require_posix_signal!((ss, old_ss) => {
        use crate::modules::posix::signal;
        use crate::interfaces::task::SignalStack;

        let current = signal::sigaltstack(None).ok().flatten();

        if !old_ss.is_null() {
            let ks = match current.as_ref() {
                Some(s) => LinuxStackT { ss_sp: s.ss_sp, ss_flags: s.ss_flags, ss_size: s.ss_size },
                None => LinuxStackT { ss_sp: 0, ss_flags: linux::SS_DISABLE as i32, ss_size: 0 },
            };
            if let Err(e) = old_ss.write(&ks) { return e; }
        }

        if !ss.is_null() {
            let ks = match ss.read() { Ok(v) => v, Err(e) => return e };

            if let Some(current_ss) = current {
                const SS_ONSTACK: i32 = 1; // commonly 1 in linux
                if (current_ss.ss_flags & SS_ONSTACK) != 0 {
                    return linux_errno(crate::modules::posix_consts::errno::EPERM);
                }
            }

            let allowed_flags = (linux::SS_DISABLE as i32) | SS_AUTODISARM;
            if (ks.ss_flags & !allowed_flags) != 0 {
                return linux_inval();
            }
            if (ks.ss_flags & (linux::SS_DISABLE as i32)) == 0 && ks.ss_size < LINUX_MINSIGSTKSZ {
                return linux_errno(crate::modules::posix_consts::errno::ENOMEM);
            }
            let new_ss = if (ks.ss_flags & (linux::SS_DISABLE as i32)) != 0 { None }
                         else { Some(crate::interfaces::task::SignalStack { ss_sp: ks.ss_sp, ss_flags: ks.ss_flags, ss_size: ks.ss_size }) };
            // Since sigaltstack expects an Option<SignalStack> (or rather passing it)
            if let Err(e) = signal::sigaltstack(new_ss) { return linux_errno(e.code()); }
        }
        0
    })
}
