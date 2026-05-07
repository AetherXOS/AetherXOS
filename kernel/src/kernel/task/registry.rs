use super::*;
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub(super) static ref TASK_REGISTRY: IrqSafeMutex<BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>> =
        IrqSafeMutex::new(BTreeMap::new());
}

pub fn register_task(task: KernelTask) {
    let id = task.id;
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_begin", Some(id.0 as u64));
    TASK_REGISTRY
        .lock()
        .insert(id, Arc::new(IrqSafeMutex::new(task)));
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_returned", Some(id.0 as u64));
}

pub fn register_task_arc(task: Arc<IrqSafeMutex<KernelTask>>) {
    let id = task.lock().id;
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_arc_begin", Some(id.0 as u64));
    TASK_REGISTRY.lock().insert(id, task);
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_arc_returned", Some(id.0 as u64));
}

pub fn unregister_task(id: TaskId) {
    crate::kernel::debug_trace::record_kernel_context("task.registry", "unregister_begin", Some(id.0 as u64));
    TASK_REGISTRY.lock().remove(&id);
    crate::kernel::debug_trace::record_kernel_context("task.registry", "unregister_returned", Some(id.0 as u64));
}

pub fn get_task(id: TaskId) -> Option<Arc<IrqSafeMutex<KernelTask>>> {
    TASK_REGISTRY.lock().get(&id).cloned()
}

pub fn task_ids_snapshot(out: &mut [TaskId]) -> usize {
    let reg = TASK_REGISTRY.lock();
    let mut written = 0usize;
    for id in reg.keys() {
        if written >= out.len() {
            break;
        }
        out[written] = *id;
        written += 1;
    }
    written
}

#[derive(Debug, Clone, Copy)]
pub struct TaskRegistrySnapshotEntry {
    pub task_id: TaskId,
    pub state: u8,
    pub process_id: usize,
    pub kernel_stack_pointer: usize,
}

impl Default for TaskRegistrySnapshotEntry {
    fn default() -> Self {
        Self {
            task_id: TaskId(0),
            state: 0,
            process_id: 0,
            kernel_stack_pointer: 0,
        }
    }
}

pub fn task_registry_snapshot(out: &mut [TaskRegistrySnapshotEntry]) -> usize {
    let reg = TASK_REGISTRY.lock();
    let mut written = 0usize;
    for (id, task_arc) in reg.iter() {
        if written >= out.len() {
            break;
        }
        let task = task_arc.lock();
        out[written] = TaskRegistrySnapshotEntry {
            task_id: *id,
            state: task.state as u8,
            process_id: task.process_id.map(|pid| pid.0).unwrap_or(0),
            kernel_stack_pointer: task.kernel_stack_pointer as usize,
        };
        written += 1;
    }
    written
}
