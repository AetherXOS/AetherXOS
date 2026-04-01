#[allow(unused_imports)]
use crate::interfaces::task::TaskId;
use crate::kernel::syscalls::SyscallFrame;
#[allow(unused_imports)]
use crate::klog_error;
#[allow(unused_imports)]
use crate::klog_trace;
#[allow(unused_imports)]
use core::sync::atomic::Ordering;

/// Checks for pending signals and redirects user-space execution to a handler if necessary.
/// Called upon return from syscall or interrupt.
pub fn check_signals(frame: &mut SyscallFrame) {
    // Signal delivery requires posix_signal + linux_compat + process_abstraction features
    #[cfg(all(
        feature = "posix_signal",
        feature = "linux_compat",
        feature = "posix_process",
        feature = "process_abstraction"
    ))]
    {
        use crate::modules::linux_compat::sig::LinuxUContext;
        use crate::modules::posix::signal::{self};

        /// Frame layout for signal delivery to user space (Linux x86_64 RT).
        #[repr(C)]
        struct LinuxRTFrame {
            pretcode: u64,
            uc: LinuxUContext,
        }

        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        let tid = TaskId(cpu.current_task.load(Ordering::Relaxed));
        if tid.0 == 0 {
            return;
        }

        let pid = match crate::kernel::launch::process_id_by_task(tid) {
            Some(p) => p.0,
            None => return,
        };

        let (sig, action) = {
            let mask = signal::sigprocmask(signal::SigmaskHow::Block, None).unwrap_or(0);
            let mut pending_lock = signal::SIGNAL_PENDING.lock();
            let set: &mut alloc::collections::BTreeSet<i32> = match pending_lock.get_mut(&pid) {
                Some(s) => s,
                None => return,
            };

            let mut found = None;
            for &s in set.iter() {
                let bit = 1u64 << ((s.saturating_sub(1)) as u64);
                if (mask & bit) == 0 {
                    found = Some(s);
                    break;
                }
            }

            if let Some(s) = found {
                set.remove(&s);
                if set.is_empty() {
                    pending_lock.remove(&pid);
                }
                let action = signal::SIGNAL_ACTIONS.lock().get(&(pid, s)).copied();
                (s, action)
            } else {
                return;
            }
        };

        let action = match action {
            Some(a) => a,
            None => {
                if sig == crate::modules::posix_consts::process::SIGKILL
                    || sig == crate::modules::posix_consts::process::SIGTERM
                {
                    let _ = crate::modules::posix::process::kill(pid, sig);
                }
                return;
            }
        };

        let handler = match action.handler {
            Some(h) => h as usize as u64,
            None => return,
        };
        if handler == 0 {
            return;
        }

        // Use rbp as the current user stack pointer reference (syscall frame doesn't store rsp)
        let mut sp = frame.rbp;
        sp = sp.saturating_sub(core::mem::size_of::<LinuxRTFrame>() as u64 + 128);
        sp &= !15u64;

        let mut uc = LinuxUContext::default();
        uc.sigmask = signal::sigprocmask(signal::SigmaskHow::Block, None).unwrap_or(0);
        uc.mcontext.r15 = frame.r15;
        uc.mcontext.r14 = frame.r14;
        uc.mcontext.r13 = frame.r13;
        uc.mcontext.r12 = frame.r12;
        uc.mcontext.rbp = frame.rbp;
        uc.mcontext.rbx = frame.rbx;
        uc.mcontext.eflags = frame.rflags;
        uc.mcontext.rip = frame.rip;
        uc.mcontext.rsp = frame.rbp; // best approximation without saved rsp

        let rt_frame = LinuxRTFrame {
            pretcode: action.restorer,
            uc,
        };
        unsafe {
            core::ptr::write_volatile(sp as *mut LinuxRTFrame, rt_frame);
        }

        frame.rip = handler;
        frame.rbp = sp; // update frame's stack reference
        frame.rdi = sig as u64;
        frame.rsi = (sp + 8) as u64;
        frame.rdx = (sp + 8) as u64;

        klog_trace!(
            "[SIGNAL] Delivered sig={} pid={} handler={:#x} sp={:#x}",
            sig,
            pid,
            handler,
            sp
        );
    }

    #[cfg(all(
        feature = "posix_signal",
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        target_arch = "x86_64"
    ))]
    {
        use crate::modules::posix::signal::{self};

        #[repr(C)]
        #[derive(Clone, Copy, Default)]
        struct LinuxStackTCompat {
            ss_sp: u64,
            ss_flags: i32,
            ss_size: u64,
        }

        #[repr(C)]
        #[derive(Clone, Copy, Default)]
        struct LinuxMContextCompat {
            r8: u64,
            r9: u64,
            r10: u64,
            r11: u64,
            r12: u64,
            r13: u64,
            r14: u64,
            r15: u64,
            rdi: u64,
            rsi: u64,
            rbp: u64,
            rbx: u64,
            rdx: u64,
            rax: u64,
            rcx: u64,
            rsp: u64,
            rip: u64,
            eflags: u64,
            cs: u16,
            gs: u16,
            fs: u16,
            ss: u16,
            err: u64,
            trapno: u64,
            oldmask: u64,
            cr2: u64,
            fpstate: u64,
            __reserved1: [u64; 8],
        }

        #[repr(C)]
        #[derive(Clone, Copy, Default)]
        struct LinuxUContextCompat {
            flags: u64,
            link: u64,
            stack: LinuxStackTCompat,
            mcontext: LinuxMContextCompat,
            sigmask: u64,
        }

        #[repr(C)]
        #[derive(Clone, Copy, Default)]
        struct LinuxRTFrameCompat {
            pretcode: u64,
            uc: LinuxUContextCompat,
        }

        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        let tid = TaskId(cpu.current_task.load(Ordering::Relaxed));
        if tid.0 == 0 {
            return;
        }

        let pid = match crate::kernel::launch::process_id_by_task(tid) {
            Some(p) => p.0,
            None => return,
        };

        let current_mask = signal::sigprocmask(signal::SigmaskHow::Block, None).unwrap_or(0);
        let (sig, action) = {
            let mut pending_lock = signal::SIGNAL_PENDING.lock();
            let set = match pending_lock.get_mut(&pid) {
                Some(s) => s,
                None => return,
            };

            let mut found = None;
            for &s in set.iter() {
                let bit = 1u64 << ((s.saturating_sub(1)) as u64);
                if (current_mask & bit) == 0 {
                    found = Some(s);
                    break;
                }
            }

            if let Some(s) = found {
                set.remove(&s);
                if set.is_empty() {
                    pending_lock.remove(&pid);
                }
                let action = signal::SIGNAL_ACTIONS.lock().get(&(pid, s)).copied();
                (s, action)
            } else {
                return;
            }
        };

        let Some(action) = action else {
            if sig == crate::modules::posix_consts::process::SIGKILL
                || sig == crate::modules::posix_consts::process::SIGTERM
            {
                let _ = crate::modules::posix::process::kill(pid, sig);
            }
            return;
        };

        let Some(handler_fn) = action.handler else {
            return;
        };
        let handler = handler_fn as usize as u64;
        if handler == 0 {
            return;
        }

        let task_arc = match crate::kernel::task::get_task(tid) {
            Some(task_arc) => task_arc,
            None => return,
        };

        let (signal_stack, blocked_mask) = {
            let task = task_arc.lock();
            let signal_stack = task.signal_stack.map(|stack| LinuxStackTCompat {
                ss_sp: stack.ss_sp,
                ss_flags: stack.ss_flags,
                ss_size: stack.ss_size,
            });
            let mut blocked = current_mask | action.mask;
            if (action.flags & crate::modules::posix_consts::signal::SA_NODEFER) == 0 {
                blocked |= 1u64 << ((sig - 1) as u64);
            }
            (signal_stack, blocked)
        };

        let mut sp = signal_stack
            .map(|stack| stack.ss_sp.saturating_add(stack.ss_size))
            .unwrap_or(frame.rbp);
        sp = sp.saturating_sub(core::mem::size_of::<LinuxRTFrameCompat>() as u64 + 128);
        sp &= !15u64;

        let rt_frame = LinuxRTFrameCompat {
            pretcode: action.restorer,
            uc: LinuxUContextCompat {
                flags: 0,
                link: 0,
                stack: signal_stack.unwrap_or_default(),
                mcontext: LinuxMContextCompat {
                    r15: frame.r15,
                    r14: frame.r14,
                    r13: frame.r13,
                    r12: frame.r12,
                    rdi: frame.rdi,
                    rsi: frame.rsi,
                    rbp: frame.rbp,
                    rbx: frame.rbx,
                    rdx: frame.rdx,
                    rax: frame.rax,
                    rcx: 0,
                    rsp: frame.rbp,
                    rip: frame.rip,
                    eflags: frame.rflags,
                    ..Default::default()
                },
                sigmask: current_mask,
            },
        };

        unsafe {
            core::ptr::write_volatile(sp as *mut LinuxRTFrameCompat, rt_frame);
        }

        let _ = signal::sigprocmask(signal::SigmaskHow::SetMask, Some(blocked_mask));
        frame.rip = handler;
        frame.rbp = sp;
        frame.rdi = sig as u64;
        frame.rsi = (sp + 8) as u64;
        frame.rdx = (sp + 8) as u64;

        klog_trace!(
            "[SIGNAL] Delivered shim sig={} pid={} handler={:#x} sp={:#x}",
            sig,
            pid,
            handler,
            sp
        );
    }

    // When signal features are disabled, nothing to do
    #[cfg(not(any(
        all(
            feature = "posix_signal",
            feature = "linux_compat",
            feature = "posix_process",
            feature = "process_abstraction"
        ),
        all(
            feature = "posix_signal",
            not(feature = "linux_compat"),
            feature = "posix_process",
            feature = "process_abstraction",
            target_arch = "x86_64"
        )
    )))]
    {
        let _ = frame;
    }
}
