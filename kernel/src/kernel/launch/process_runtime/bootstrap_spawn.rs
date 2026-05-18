use super::*;
use core::sync::atomic::Ordering;
use crate::interfaces::task::TaskId;
use crate::kernel::launch::process_runtime::bootstrap_dispatch::record_launch_image_preview;

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
    bootstrap_spawn_observability::log_spawn_begin(
        process_name,
        image.len(),
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        interpreter_image.as_ref().map(|img| img.len()).unwrap_or(0),
    );
    record_launch_image_preview(image);
    let boot_image = BootImageRecord::Owned(image.to_vec());
    let result = spawn_bootstrap_from_image_record(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        interpreter_image,
    );
    bootstrap_spawn_observability::log_spawn_result(process_name, &result, image.len());
    result
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
    SPAWN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "spawn_record_begin",
            Some(boot_image.as_slice().len() as u64),
            false,
        );
    }

    bootstrap_spawn_utils::validate_spawn_request(process_name, &boot_image)?;

    let name_str = alloc::string::String::from_utf8_lossy(process_name);
    bootstrap_spawn_utils::log_spawn_record(&name_str, boot_image.as_slice().len(), priority, deadline, burst_time, kernel_stack_top);

    let (process, _) = bootstrap_spawn_helpers::create_process_with_cr3(name_str.as_bytes());
    let process_id = process.id;
    let task_id = TaskId(process_id.0);

    let image_bytes = boot_image.as_slice();

    let task = bootstrap_spawn_task::build_bootstrap_task(
        &process,
        image_bytes,
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        _interpreter_image,
    )?;

    publish_bootstrap_process_and_task(process, task, task_id, boot_image)
}
