use super::*;
use alloc::sync::Arc;
use crate::interfaces::task::TaskId;
use crate::kernel::process::Process;

#[cfg(feature = "process_abstraction")]
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
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let offset = x86_64::VirtAddr::new(hhdm);
    let lvl4 = unsafe { &mut *(((process.cr3.as_u64() as u64) + hhdm) as *mut x86_64::structures::paging::PageTable) };
    let mut page_manager = crate::kernel::memory::paging::PageManager {
        mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
        physical_memory_offset: offset,
    };
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

    if let Some(interp) = interpreter_image {
        let interp_prepared = crate::kernel::module_loader::materialize_process_image(
            process,
            &interp,
            &mut page_manager,
            &mut frame_allocator,
        ).map_err(|_| LaunchError::LoaderFailed)?;

        process.set_interpreter_base(interp_prepared.load_plan.aslr_base);
        process.set_runtime_entry(Some(interp_prepared.load_plan.entry + interp_prepared.load_plan.aslr_base));
    }

    Ok(prepared)
}
