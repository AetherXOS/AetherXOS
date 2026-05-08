/// Task Management & Signal Delivery — Production Grade
///
/// # Modular Structure
/// - `registry`: Task storage and lookup.
/// - `lifecycle`: Task creation (cloning) and state management.
/// - `signals`: Task-level signal delivery logic.
/// - `scheduling`: Blocking and waking tasks via the scheduler.

pub use crate::interfaces::task::*;
use alloc::sync::Arc;
use crate::kernel::sync::IrqSafeMutex;
use crate::core::log;

pub use crate::interfaces::task::KernelTask as Task;


pub fn spawn_task(task: Arc<IrqSafeMutex<KernelTask>>) -> TaskId {
    let id = task.lock().id;
    register_task_arc(task.clone());
    
    // PHASE 6 TASK 7: Initialize task in scheduler
    if let Err(e) = crate::kernel_runtime::syscall_integration::on_task_spawn(id) {
        log::warn(&format!("Failed to initialize task {} in scheduler: {}", id.0, e));
    }
    
    wake_task(id);
    id
}

pub fn alloc_tid() -> TaskId {
    static NEXT_TID: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1000);
    TaskId(NEXT_TID.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
}

/// Get the ID of the currently executing task on the local CPU.
pub fn current_task_id() -> TaskId {
    unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| TaskId(cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)))
            .unwrap_or(TaskId(0))
    }
}

/// Get the security context of the currently executing task.
pub fn current_security_context() -> crate::interfaces::security::SecurityContext {
    get_task(current_task_id())
        .map(|t| t.lock().security_ctx)
        .unwrap_or(crate::interfaces::security::SecurityContext::kernel())
}


pub mod registry;
pub mod lifecycle;
pub mod signals;
pub mod scheduling;
pub mod kthread;

pub use registry::{
    register_task, register_task_arc, unregister_task, get_task, 
    task_ids_snapshot, task_registry_snapshot, TaskRegistrySnapshotEntry
};
pub use signals::check_and_deliver_signals;
pub use lifecycle::{clone_current_task};
pub use scheduling::{suspend_current_task, suspend_current_task_with_mask, wake_task, wake_tasks};

pub(crate) fn clear_robust_list_for_tid(tid: usize) {
    crate::kernel::syscalls::clear_robust_list_for_tid(tid);
}
