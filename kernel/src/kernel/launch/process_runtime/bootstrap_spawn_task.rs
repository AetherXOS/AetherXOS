use super::*;
use alloc::sync::Arc;
use crate::interfaces::task::TaskId;
use crate::kernel::process::Process;

#[cfg(feature = "process_abstraction")]
pub fn build_bootstrap_task(
    process: &Arc<Process>,
    image_bytes: &[u8],
    task_id: TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>, LaunchError> {
    #[cfg(feature = "paging_enable")]
    {
        return bootstrap_spawn_materialize::materialize_and_prepare_task_paging(
            process,
            image_bytes,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            interpreter_image,
        );
    }

    #[cfg(not(feature = "paging_enable"))]
    {
        crate::kernel::module_loader::build_process_bootstrap_task(
            process,
            image_bytes,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        )
        .map_err(|_| LaunchError::LoaderFailed)
    }
}
