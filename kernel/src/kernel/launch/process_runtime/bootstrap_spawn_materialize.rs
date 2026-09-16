#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
use alloc::sync::Arc;
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
use crate::interfaces::task::TaskId;
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
use crate::kernel::process::Process;

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub fn materialize_and_prepare_task_paging(
    process: &Arc<Process>,
    image_bytes: &[u8],
    task_id: TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>, LaunchError> {
    let (_offset, mut page_manager) = bootstrap_spawn_paging::build_page_manager(process);
    let mut frame_allocator = crate::hal::HAL::create_frame_allocator();

    let prepared = crate::kernel::module_loader::materialize_and_build_process_bootstrap_task(
        process,
        image_bytes,
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        &mut page_manager,
        &mut frame_allocator,
    ).map_err(|_| LaunchError::LoaderFailed)?;

    bootstrap_spawn_paging::materialize_interpreter_image(
        process,
        interpreter_image,
        &mut page_manager,
        &mut frame_allocator,
    )?;

    Ok(prepared)
}
