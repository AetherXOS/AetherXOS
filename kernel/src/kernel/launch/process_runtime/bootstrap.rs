use super::*;
use core::sync::atomic::Ordering;

#[cfg(feature = "process_abstraction")]
#[cfg(feature = "paging_enable")]
fn preflight_bootstrap_image(
    process_name: &[u8],
    image: &[u8],
) -> Result<(), LaunchError> {
    if let Err(err) = crate::kernel::module_loader::preflight_module_image(image) {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "preflight_failed",
            Some(0),
            false,
        );
        crate::klog_warn!(
            "[LAUNCH] preflight rejected bootstrap image name='{}' bytes={} error={:?}",
            alloc::string::String::from_utf8_lossy(process_name),
            image.len(),
            err,
        );
        VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(LaunchError::LoaderFailed);
    }
    Ok(())
}

#[cfg(feature = "process_abstraction")]
#[cfg(not(feature = "paging_enable"))]
fn preflight_bootstrap_image(
    process_name: &[u8],
    image: &[u8],
) -> Result<crate::kernel::module_loader::ModuleImageSnapshot, LaunchError> {
    match crate::kernel::module_loader::snapshot_module_image(image) {
        Ok(snapshot) => Ok(snapshot),
        Err(err) => {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "preflight_failed",
                Some(err as u64),
                false,
            );
            crate::klog_warn!(
                "[LAUNCH] snapshot rejected bootstrap image name='{}' bytes={} error={:?}",
                alloc::string::String::from_utf8_lossy(process_name),
                image.len(),
                err,
            );
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
            Err(LaunchError::LoaderFailed)
        }
    }
}

#[cfg(feature = "process_abstraction")]
pub fn publish_bootstrap_process_and_task(
    process: alloc::sync::Arc<Process>,
    task: alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    task_id: TaskId,
    registry_boot_image: BootImageRecord,
) -> Result<(usize, usize), LaunchError> {
    crate::kernel::launch::process_runtime::bootstrap_publish::publish_bootstrap_process_and_task(
        process,
        task,
        task_id,
        registry_boot_image,
    )
}

#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<(usize, usize), LaunchError> {
    crate::kernel::launch::process_runtime::bootstrap_spawn::spawn_bootstrap_from_image(
        process_name,
        image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        interpreter_image,
    )
}


#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_image_record(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    _interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<(usize, usize), LaunchError> {
    crate::kernel::launch::process_runtime::bootstrap_spawn::spawn_bootstrap_from_image_record(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        _interpreter_image,
    )
}
