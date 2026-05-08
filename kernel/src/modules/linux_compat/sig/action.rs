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
///
/// On entry via `sa_restorer`, RSP points at the `pretcode` field of the
/// `rt_sigframe` (i.e. the very start of the frame we built in `setup_linux_sigframe`).
/// Layout:
///   [RSP+ 0] pretcode (8 bytes)
///   [RSP+ 8] sig      (4 bytes)
///   [RSP+12] _pad     (4 bytes)
///   [RSP+16] *siginfo  (8 bytes)
///   [RSP+24] *uc       (8 bytes)
///   [RSP+32] LinuxSiginfo (128 bytes)
///   [RSP+160] LinuxUContext
pub fn sys_linux_rt_sigreturn(frame: &mut SyscallFrame) -> usize {
    crate::require_posix_signal!((frame) => {
        // The uc (ucontext) lives 160 bytes past the start of the rt_sigframe.
        // rt_sigframe = { pretcode(8) + sig(4) + pad(4) + pinfo(8) + puc(8) } = 32 bytes header
        // + LinuxSiginfo (128 bytes) = 160 bytes before uc.
        const UC_OFFSET: usize = 8 + 4 + 4 + 8 + 8 + core::mem::size_of::<LinuxSiginfo>();

        let uc_addr = (frame.rsp as usize).wrapping_add(UC_OFFSET);
        let uctx = match UserPtr::<LinuxUContext>::new(uc_addr).read() {
            Ok(c) => c,
            Err(_) => {
                // Fallback: try reading directly at RSP (old-style frame)
                match UserPtr::<LinuxUContext>::new(frame.rsp as usize).read() {
                    Ok(c) => c,
                    Err(_) => return linux_fault(),
                }
            }
        };

        let m = &uctx.mcontext;

        // Restore ALL general-purpose registers saved in mcontext
        frame.r15    = m.r15;
        frame.r14    = m.r14;
        frame.r13    = m.r13;
        frame.r12    = m.r12;
        frame.r11    = m.r11;
        frame.r10    = m.r10;
        frame.r9     = m.r9;
        frame.r8     = m.r8;
        frame.rbp    = m.rbp;
        frame.rbx    = m.rbx;
        frame.rax    = m.rax;
        frame.rcx    = m.rcx;
        frame.rdx    = m.rdx;
        frame.rsi    = m.rsi;
        frame.rdi    = m.rdi;
        frame.rip    = m.rip;
        frame.rsp    = m.rsp;
        // Preserve only user-space flags bits (mask out kernel-only bits like IOPL, NT)
        let safe_flags = m.eflags & 0x0003_7FD5; // allow: CF,PF,AF,ZF,SF,TF,IF,DF,OF,AC
        frame.rflags = safe_flags;

        // Restore signal mask from uc_sigmask
        use crate::modules::posix::signal::{self, SigmaskHow};
        let _ = signal::sigprocmask(SigmaskHow::SetMask, Some(uctx.sigmask));

        // Clear signal_stack_active if we were on alternate stack
        #[cfg(feature = "process_abstraction")]
        {
            if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
                let tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
                if let Some(t) = crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid)) {
                    t.lock().signal_stack_active = false;
                }
            }
        }

        // Return saved rax (syscall return value from before signal delivery)
        m.rax as usize
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
