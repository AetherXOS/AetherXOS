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

pub use crate::interfaces::task::KernelTask as Task;


pub fn spawn_task(task: Arc<IrqSafeMutex<KernelTask>>) -> TaskId {
    let id = task.lock().id;
    register_task_arc(task.clone());
    wake_task(id);
    id
}

pub fn alloc_tid() -> TaskId {
    static NEXT_TID: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1000);
    TaskId(NEXT_TID.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
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
