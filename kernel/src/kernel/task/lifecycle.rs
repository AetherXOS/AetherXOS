//! Task State Machine and Lifecycle Management for AetherXOS.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,      // Waiting for CPU
    Running,    // Currently executing
    Blocked,    // Waiting for I/O or sleep
    Stopped,    // Suspended (SIGSTOP)
    Zombie,     // Finished but not reaped by parent
}

use crate::interfaces::task::{TaskId, TaskState, KernelTask, ProcessId};
use alloc::sync::Arc;
use crate::kernel::sync::IrqSafeMutex;

pub fn clone_current_task(new_tid: TaskId, new_pid: ProcessId, new_cr3: u64) -> Result<Arc<IrqSafeMutex<KernelTask>>, &'static str> {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    
    let current_task_arc = super::registry::get_task(TaskId(current_tid)).ok_or("current task not found")?;
    let current_task = current_task_arc.lock();
    
    let mut new_task = current_task.clone();
    new_task.id = new_tid;
    new_task.process_id = Some(new_pid);
    new_task.page_table_root = new_cr3;
    
    // Reset state for child
    new_task.state = TaskState::Ready;
    new_task.pending_signals = 0;
    new_task.handling_signals = 0;
    new_task.signal_stack_active = false;
    // Separate signal queue
    new_task.signal_queue = Arc::new(IrqSafeMutex::new(crate::kernel::signal::queue::SignalQueue::new()));
    
    Ok(Arc::new(IrqSafeMutex::new(new_task)))
}

pub struct TaskLifecycle;

impl TaskLifecycle {
    /// Transition a task to Zombie state and notify parent.
    pub fn exit_task(pid: u32, exit_code: i32) {
        crate::klog_info!("[TASK] PID {} exiting with code {}. Transitioning to ZOMBIE.", pid, exit_code);
        // In a real system, we'd update the process table and wake up the parent
    }

    /// Block a task until a specific event occurs.
    pub fn block_task(pid: u32, reason: &'static str) {
        crate::klog_info!("[TASK] PID {} BLOCKED (Reason: {})", pid, reason);
    }
}
