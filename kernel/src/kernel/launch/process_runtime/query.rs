use super::*;
use alloc::sync::Arc;
use crate::interfaces::task::{ProcessId, TaskId};
use crate::kernel::process::Process;
use crate::kernel::cpu_local::CpuLocal;
use core::sync::atomic::Ordering;

#[cfg(feature = "process_abstraction")]
pub fn process_count() -> usize {
    PROCESS_REGISTRY.lock().len()
}

#[cfg(feature = "process_abstraction")]
pub fn process_ids_snapshot(out: &mut [ProcessId]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        out[written] = entry.process_id;
        written += 1;
    }
    written
}

#[cfg(feature = "process_abstraction")]
pub fn launch_registry_snapshot(out: &mut [LaunchRegistrySnapshotEntry]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        let (_, image_pages, _, _) = entry.process.image_state();
        let (_, mapped_pages) = entry.process.mapping_state();
        out[written] = LaunchRegistrySnapshotEntry {
            process_id: entry.process_id,
            task_id: entry.task_id,
            stage: entry.stage.as_usize(),
            image_pages,
            mapped_pages,
        };
        written += 1;
    }
    written
}

#[cfg(feature = "process_abstraction")]
pub fn process_image_state(process_id: ProcessId) -> Option<(usize, usize, usize)> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| {
            let (entry_point, image_pages, image_segments, _) = entry.process.image_state();
            (entry_point, image_pages, image_segments)
        })
}

#[cfg(feature = "process_abstraction")]
pub fn process_arc_by_id(process_id: ProcessId) -> Option<Arc<Process>> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.process.clone())
}

#[cfg(feature = "process_abstraction")]
pub fn process_mapping_state(process_id: ProcessId) -> Option<(usize, usize)> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.process.mapping_state())
}

#[cfg(feature = "process_abstraction")]
pub fn process_id_by_task(task_id: TaskId) -> Option<ProcessId> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.task_id == task_id)
        .map(|entry| entry.process_id)
}

#[cfg(feature = "process_abstraction")]
pub fn current_process_arc() -> Option<Arc<Process>> {
    let tid =
        unsafe { CpuLocal::try_get().map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed))) }?;

    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|e| e.task_id == tid)
        .map(|e| e.process.clone())
}
