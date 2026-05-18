use super::super::*;
use alloc::sync::Arc;
use crate::interfaces::task::{ProcessId, TaskId};
use crate::kernel::process::Process;

#[cfg(feature = "process_abstraction")]
pub fn process_image_state(process_id: ProcessId) -> Option<(usize, usize, usize)> {
    let entry = match query_helpers::find_process_entry(process_id) {
        Some(entry) => entry,
        None => {
            query_helpers::log_query_miss("process_image_state", process_id);
            return None;
        }
    };
    let (entry_point, image_pages, image_segments, _) = entry.process.image_state();
    Some((entry_point, image_pages, image_segments))
}

#[cfg(feature = "process_abstraction")]
pub fn process_arc_by_id(process_id: ProcessId) -> Option<Arc<Process>> {
    let entry = match query_helpers::find_process_entry(process_id) {
        Some(entry) => entry,
        None => {
            query_helpers::log_query_miss("process_arc_by_id", process_id);
            return None;
        }
    };
    Some(query_helpers::process_arc_from_entry(&entry))
}

#[cfg(feature = "process_abstraction")]
pub fn process_mapping_state(process_id: ProcessId) -> Option<(usize, usize)> {
    let entry = match query_helpers::find_process_entry(process_id) {
        Some(entry) => entry,
        None => {
            query_helpers::log_query_miss("process_mapping_state", process_id);
            return None;
        }
    };
    Some(entry.process.mapping_state())
}

#[cfg(feature = "process_abstraction")]
pub fn process_id_by_task(task_id: TaskId) -> Option<ProcessId> {
    crate::kernel::task::get_task(task_id).and_then(|task_arc| {
        task_arc.lock().process_id
    })
}

#[cfg(feature = "process_abstraction")]
pub fn current_process_arc() -> Option<Arc<Process>> {
    let pid = query_helpers::current_process_id()?;

    if pid.0 == 0 {
        return None;
    }
    process_arc_by_id(pid)
}
