use super::*;
use core::sync::atomic::Ordering;
use crate::modules::posix_consts::process;

/// Linux signal delivery flags (sa_flags bits)
const SA_NOCLDSTOP:  u64 = 1 << 0;
const SA_NOCLDWAIT:  u64 = 1 << 1;
const SA_SIGINFO:    u64 = 1 << 2;
const SA_ONSTACK:    u64 = 1 << 3;
const SA_RESTART:    u64 = 1 << 4;
const SA_NODEFER:    u64 = 1 << 6;
const SA_RESETHAND:  u64 = 1 << 7;

/// Check the current task's signal queue and deliver the first unmasked pending signal.
///
/// Called on the return path from every syscall and on timer-driven preemption.
/// This is the core of Linux signal delivery semantics:
///   - Masked signals are kept in the queue (not discarded)
///   - SA_RESETHAND: reset handler to SIG_DFL before invoking
///   - SA_NODEFER: don't block the signal during handler
///   - SIGKILL/SIGSTOP cannot be caught or masked; always fatal
///   - SIG_IGN: discard silently
///   - SIG_DFL: apply default action (typically terminate for most signals)
pub fn check_and_deliver_signals() {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let cur_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));

    if cur_tid.0 == 0 {
        return;
    }

    let Some(task_arc) = get_task(cur_tid) else {
        return;
    };

    // Drain queue until we find an unmasked signal or exhaust it
    loop {
        let (sig_info, signal_mask, process_id) = {
            let t = task_arc.lock();
            let sig = t.signal_queue.lock().pop();
            (sig, t.signal_mask, t.process_id)
        };

        let Some(sig_info) = sig_info else {
            return; // Queue empty
        };

        let sig_nr = sig_info.nr as i32;
        let bit = if sig_nr > 0 && sig_nr <= 64 { 1u64 << (sig_nr - 1) } else { 0 };

        // SIGKILL and SIGSTOP cannot be blocked
        let is_uncatchable = sig_nr == 9 || sig_nr == 19; // SIGKILL=9, SIGSTOP=19

        // Check signal mask (skip uncatchable signals)
        if !is_uncatchable && (signal_mask & bit) != 0 {
            // Re-queue the masked signal and stop processing
            task_arc.lock().signal_queue.lock().push(sig_info);
            return;
        }

        let Some(_pid) = process_id else {
            // No process context - apply default (terminate)
            apply_default_action(cur_tid, sig_nr);
            return;
        };

        // Get signal action
        #[cfg(all(feature = "posix_signal", feature = "linux_compat"))]
        {
            use crate::modules::posix::signal::{SIGNAL_ACTIONS, SignalAction};

            let action_opt = {
                let handlers = SIGNAL_ACTIONS.lock();
                handlers.get(&(_pid.0 as usize, sig_nr)).copied()
            };

            match action_opt {
                None => {
                    // No handler registered → default action
                    apply_default_action(cur_tid, sig_nr);
                    return;
                }
                Some(action) => {
                    // SIG_IGN (handler address = 1 in Linux convention)
                    let handler_addr = action.handler.map(|h| h as u64).unwrap_or(0);
                    if handler_addr == 1 {
                        // SIG_IGN - silently discard
                        continue;
                    }
                    if handler_addr == 0 {
                        // SIG_DFL
                        apply_default_action(cur_tid, sig_nr);
                        return;
                    }

                    // Inject the signal frame
                    let mut task = task_arc.lock();

                    // SA_RESETHAND: reset to SIG_DFL before calling handler
                    if (action.flags as u64) & SA_RESETHAND != 0 {
                        let mut handlers = SIGNAL_ACTIONS.lock();
                        handlers.remove(&(_pid.0 as usize, sig_nr));
                    }

                    match crate::modules::linux_compat::sig::setup_linux_sigframe(
                        &mut task,
                        sig_nr,
                        &action,
                    ) {
                        Ok(_new_rsp) => {
                            crate::klog_trace!(
                                "signal: delivered sig={} handler={:#x} tid={}",
                                sig_nr, handler_addr, cur_tid.0
                            );
                        }
                        Err(reason) => {
                            crate::klog_warn!(
                                "signal: frame injection failed: {} sig={} tid={}",
                                reason, sig_nr, cur_tid.0
                            );
                            // Fallback: set RIP directly (no sigreturn possible)
                            #[cfg(target_arch = "x86_64")]
                            {
                                task.context.set_instruction_pointer(handler_addr);
                                task.context.set_arg_register_0(sig_nr as u64);
                            }
                        }
                    }
                    return; // Deliver one signal per syscall return
                }
            }
        }

        #[cfg(not(all(feature = "posix_signal", feature = "linux_compat")))]
        {
            apply_default_action(cur_tid, sig_nr);
            return;
        }
    }
}

/// Apply the default signal action for a given signal number.
/// Most signals terminate the process; SIGCHLD and SIGURG are ignored by default.
fn apply_default_action(tid: TaskId, sig_nr: i32) {
    // Signals with SIG_IGN as default
    const IGN_BY_DEFAULT: &[i32] = &[
        process::SIGCHLD,
        process::SIGURG,
        process::SIGWINCH,
        process::SIGIO,
    ];
    
    if IGN_BY_DEFAULT.contains(&sig_nr) {
        return;
    }

    // SIGSTOP / SIGTSTP / SIGTTIN / SIGTTOU - stop the process
    if matches!(sig_nr, 
        process::SIGSTOP | 
        process::SIGTSTP | 
        process::SIGTTIN | 
        process::SIGTTOU) {
        // Process stop (TASK_STOPPED state)
        if let Some(task) = crate::kernel::task::get_task(tid) {
            let mut locked = task.lock();
            locked.state = crate::interfaces::task::state::TaskState::Stopped;
            crate::klog_trace!("signal: SIGSTOP-family sig={} tid={} -> TASK_STOPPED", sig_nr, tid.0);
        }
        return;
    }

    // SIGCONT - continue stopped process
    if sig_nr == process::SIGCONT {
        if let Some(task) = crate::kernel::task::get_task(tid) {
            let mut locked = task.lock();
            if locked.state == crate::interfaces::task::state::TaskState::Stopped {
                locked.state = crate::interfaces::task::state::TaskState::Ready;
                crate::klog_trace!("signal: SIGCONT sig={} tid={} -> TASK_READY", sig_nr, tid.0);
            }
        }
        return;
    }

    // All other signals: terminate
    crate::klog_info!("signal: default action TERM sig={} tid={}", sig_nr, tid.0);
    let _ = crate::kernel::launch::terminate_task(tid);
}
