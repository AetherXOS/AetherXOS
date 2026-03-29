use crate::interfaces::TaskId;
use crate::interfaces::task::ProcessId;
use crate::kernel::task::{get_task, check_and_deliver_signals};
use crate::kernel::process_registry::get_process;

/// Send signal `sig` to all threads of `pid`.
pub fn sys_kill(pid: ProcessId, sig: i32) -> Result<(), &'static str> {
    if sig < 0 || sig >= 64 {
        return Err("invalid signal number");
    }

    let process = get_process(pid).ok_or("process not found")?;
    let threads = process.threads.lock().clone();

    if threads.is_empty() {
        return Err("no threads in process");
    }

    let bit = 1u64 << sig;
    for tid in threads {
        if let Some(task_arc) = get_task(tid) {
            task_arc.lock().pending_signals |= bit;
        }
    }

    // Attempt immediate delivery if we are in that process already.
    check_and_deliver_signals();
    Ok(())
}

/// Register a user-space signal handler for the calling process.
pub fn sys_sigaction(sig: i32, handler: u64) -> Result<(), &'static str> {
    if sig < 0 || sig >= 64 {
        return Err("invalid signal number");
    }

    let current_task_id = unsafe {
        crate::kernel::cpu_local::CpuLocal::get()
            .current_task
            .load(core::sync::atomic::Ordering::Relaxed)
    };

    let task_arc = get_task(TaskId(current_task_id)).ok_or("current task not found")?;
    let pid = task_arc.lock().process_id.ok_or("task has no process")?;

    let process = get_process(pid).ok_or("process not found")?;
    process.signal_handlers.lock().insert(sig, handler);
    Ok(())
}

/// Block or unblock signals (sigprocmask).
/// `how`:  0 = SIG_BLOCK, 1 = SIG_UNBLOCK, 2 = SIG_SETMASK
/// `sigset`: bitmask of signals (signal N = bit N).
pub fn sys_sigprocmask(how: u32, sigset: u64) -> Result<u64, &'static str> {
    let current_task_id = unsafe {
        crate::kernel::cpu_local::CpuLocal::get()
            .current_task
            .load(core::sync::atomic::Ordering::Relaxed)
    };

    let task_arc = get_task(TaskId(current_task_id)).ok_or("current task not found")?;
    let mut task = task_arc.lock();
    let old_mask = task.signal_mask;

    task.signal_mask = match how {
        0 => old_mask | sigset,       // SIG_BLOCK
        1 => old_mask & !sigset,      // SIG_UNBLOCK
        2 => sigset,                  // SIG_SETMASK
        _ => return Err("invalid how value"),
    };

    Ok(old_mask)
}

/// Temporarily replace the signal stack for the calling task.
pub fn sys_sigaltstack(ss_sp: u64, ss_size: u64) -> Result<(), &'static str> {
    let current_task_id = unsafe {
        crate::kernel::cpu_local::CpuLocal::get()
            .current_task
            .load(core::sync::atomic::Ordering::Relaxed)
    };

    let task_arc = get_task(TaskId(current_task_id)).ok_or("current task not found")?;
    let mut task = task_arc.lock();
    task.signal_stack = Some(crate::interfaces::task::SignalStack {
        ss_sp,
        ss_flags: 0,
        ss_size,
    });
    Ok(())
}
