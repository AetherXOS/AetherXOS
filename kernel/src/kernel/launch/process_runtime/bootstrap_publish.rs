use super::*;
use core::sync::atomic::Ordering;
use crate::interfaces::task::TaskId;

#[cfg(feature = "process_abstraction")]
pub fn publish_bootstrap_process_and_task(
    process: alloc::sync::Arc<Process>,
    task: alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    task_id: TaskId,
    registry_boot_image: BootImageRecord,
) -> Result<(usize, usize), LaunchError> {
    bootstrap_publish_helpers::register_and_enqueue(process, task, task_id, registry_boot_image)?;
    let process_id = task_id.0;
    LAST_TASK_ID.store(task_id.0, Ordering::Relaxed);
    SPAWN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "spawn_returned",
        Some(process_id as u64),
        false,
    );
    Ok((process_id, task_id.0))
}
