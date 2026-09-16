use super::super::*;
use super::super::terminate_helpers;

use core::sync::atomic::Ordering;

#[cfg(feature = "process_abstraction")]
pub fn terminate_process_with_status(process_id: ProcessId, status: i32) -> bool {
    TERMINATE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    terminate_helpers::log_process_exit_requested(process_id, status);

    let (task_id, process_arc) = {
        let mut registry = PROCESS_REGISTRY.lock();
        let Some(index) = registry
            .iter()
            .position(|entry| entry.process_id == process_id)
        else {
            TERMINATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::klog_warn!(
                "process exit ignored: pid={} status={} reason=registry-miss",
                process_id.0,
                status,
            );
            return false;
        };
        let entry = registry.remove(index);
        (entry.task_id, entry.process)
    };

    terminate_helpers::log_process_exit_accepted(process_id, task_id, status);

    let shared_object_fini =
        crate::kernel::dynamic_linker::api::drain_pending_shared_object_fini_reports_for_process(
            process_id,
        );
    terminate_helpers::log_deferred_fini_reports(process_id, &process_arc, &shared_object_fini);

    let runtime_contract = process_arc.runtime_contract_snapshot();
    terminate_helpers::log_runtime_contract_exit(process_id, &runtime_contract, status);

    process_arc.mark_exited(status);
    process_arc.clear_runtime_contract();
    wrappers::finalize_task_user_exit_state(task_id);

    terminate_helpers::cleanup_task_on_all_cpus(task_id);

    TERMINATE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_task(task_id: TaskId) -> bool {
    TERMINATE_BY_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let task_arc = crate::kernel::task::get_task(task_id);
    let Some(task_arc) = task_arc else {
        TERMINATE_BY_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
        return false;
    };

    let process_id_opt = task_arc.lock().process_id;

    if let Some(process_id) = process_id_opt {
        if let Some(process) = crate::kernel::process::registry::get_process(process_id) {
            let mut threads = process.threads.lock();
            if let Some(pos) = threads.iter().position(|&tid| tid == task_id) {
                threads.remove(pos);
            }

            if threads.is_empty() {
                drop(threads);
                return terminate_process_with_status(process_id, 0);
            }
        }
    }

    // Single thread exit logic
    wrappers::finalize_task_user_exit_state(task_id);

    let cpus = crate::hal::smp::CPUS.lock();
    for cpu in cpus.iter() {
        let mut scheduler = cpu.scheduler.lock();
        scheduler.remove_task(task_id);
    }

    TERMINATE_BY_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}
