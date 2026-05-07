use super::*;
use core::sync::atomic::Ordering;

pub fn check_and_deliver_signals() {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let cur_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));

    if cur_tid.0 == 0 {
        return;
    }

    let Some(task_arc) = get_task(cur_tid) else {
        return;
    };

    let (signal, signal_mask, process_id) = {
        let t = task_arc.lock();
        let sig = t.signal_queue.lock().pop();
        (sig, t.signal_mask, t.process_id)
    };

    let Some(sig_info) = signal else {
        return;
    };

    let sig_nr = sig_info.nr;
    let bit = 1u64 << (sig_nr - 1);
    
    if (signal_mask & bit) != 0 {
        // Signal is masked, put it back or handle accordingly
        // (For now, we'll just skip masked signals)
        return;
    }

    let Some(pid) = process_id else { return };
    #[cfg(feature = "process_abstraction")]
    let Some(proc) = crate::kernel::process_registry::get_process(pid) else {
        return;
    };

    {
        let handlers = proc.signal_handlers.lock();
        let Some(&handler_vaddr) = handlers.get(&(sig_nr as i32)) else {
            return;
        };

        let mut task = task_arc.lock();
        // Perform Production-Grade Linux Frame Injection
        #[cfg(all(feature = "posix_signal", feature = "linux_compat"))]
        {
            use crate::modules::posix::signal::SIGNAL_ACTIONS;
            let action = SIGNAL_ACTIONS.lock().get(&(pid.0 as usize, sig_nr as i32)).copied();
            
            if let Some(act) = action {
                if let Ok(_new_rsp) = crate::modules::linux_compat::sig::setup_linux_sigframe(&mut task, sig_nr as i32, &act) {
                    // Task is now set up to run the handler on next switch
                    crate::klog_trace!(
                        "signal: high-fidelity delivery sig={} handler={:#x} tid={}",
                        sig_nr, handler_vaddr, cur_tid.0
                    );
                }
            }
        }
        #[cfg(not(all(feature = "posix_signal", feature = "linux_compat")))]
        {
            #[cfg(target_arch = "x86_64")]
            {
                task.context.rip = handler_vaddr;
            }
        }
    }
}
