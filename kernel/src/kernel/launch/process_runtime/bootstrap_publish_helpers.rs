//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
use super::*;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use crate::interfaces::task::TaskId;
use crate::kernel::process::Process;
use crate::kernel::sync::IrqSafeMutex;
use crate::interfaces::KernelTask;
use crate::kernel::cpu_local::CpuLocal;

#[cfg(feature = "process_abstraction")]
pub fn register_and_enqueue(
    process: Arc<Process>,
    task: Arc<IrqSafeMutex<KernelTask>>,
    task_id: TaskId,
    registry_boot_image: BootImageRecord,
) -> Result<(), LaunchError> {
    let process_id = process.id.0;
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "publish_begin",
        Some(process_id as u64),
        false,
    );

    let proc_ref = support::register_process(process);
    support::register_process_with_task_image(proc_ref, task_id, registry_boot_image);

    let cpu = match unsafe { CpuLocal::try_get() } {
        Some(cpu) => cpu,
        None => {
            ENQUEUE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(LaunchError::SchedulerUnavailable);
        }
    };

    crate::kernel::task::register_task_arc(task.clone());
    let mut scheduler = cpu.scheduler.lock();
    scheduler.add_task(task.clone());

    crate::kernel::rt_preemption::request_forced_reschedule();
    Ok(())
}

