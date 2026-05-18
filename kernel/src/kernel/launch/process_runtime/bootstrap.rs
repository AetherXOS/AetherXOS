use super::*;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use crate::interfaces::task::{TaskId};
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::process::Process;
use crate::kernel::launch::process_runtime::bootstrap_dispatch::record_launch_image_preview;
use crate::observability_launch;

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
    let process_id = process.id.0;
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "publish_begin",
        Some(process_id as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register process begin\n",
    );
    let proc_ref = support::register_process(process);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register process returned\n",
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap registry image begin\n",
    );
    support::register_process_with_task_image(proc_ref, task_id, registry_boot_image);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap registry image returned\n",
    );
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "registry_image_returned",
        Some(task_id.0 as u64),
        false,
    );

    let cpu = match unsafe { CpuLocal::try_get() } {
        Some(cpu) => cpu,
        None => {
            ENQUEUE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(LaunchError::SchedulerUnavailable);
        }
    };

    crate::kernel::task::register_task_arc(task.clone());
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "register_task_returned",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register task returned\n",
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler lock begin\n",
    );
    let mut scheduler = cpu.scheduler.lock();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler lock returned\n",
    );
    scheduler.add_task(task.clone());
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "scheduler_add_returned",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler add returned\n",
    );

    crate::kernel::rt_preemption::request_forced_reschedule();
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "forced_reschedule_requested",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap forced reschedule requested\n",
    );

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
