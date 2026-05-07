use super::*;
use super::support::*;

use core::sync::atomic::Ordering;
use crate::interfaces::task::{ProcessId, TaskId, TaskState};
use crate::kernel::cpu_local::CpuLocal;
use crate::klog_info;

#[cfg(feature = "process_abstraction")]
pub fn claim_next_launch_context() -> Option<LaunchContext> {
    CLAIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    for entry in registry.iter_mut() {
        if entry.stage == LaunchStage::Pending {
            entry.stage = LaunchStage::Claimed;
            entry.stage_epoch = now_epoch;
            CLAIM_SUCCESS.fetch_add(1, Ordering::Relaxed);
            return Some(build_context(
                entry.process_id,
                &entry.process,
                entry.task_id,
            ));
        }
    }

    CLAIM_FAILURES.fetch_add(1, Ordering::Relaxed);
    None
}

#[cfg(feature = "process_abstraction")]
pub fn acknowledge_launch_context_typed(process_id: ProcessId, success: bool) -> bool {
    HANDOFF_ACK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    let Some(entry) = registry
        .iter_mut()
        .find(|entry| entry.process_id == process_id)
    else {
        HANDOFF_ACK_FAILURES.fetch_add(1, Ordering::Relaxed);
        return false;
    };

    entry.stage = if success {
        LaunchStage::Ready
    } else {
        LaunchStage::Pending
    };
    entry.stage_epoch = now_epoch;
    HANDOFF_ACK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "process_abstraction")]
pub fn launch_context_stage_typed(process_id: ProcessId) -> Option<usize> {
    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.stage.as_usize())
}

#[cfg(feature = "process_abstraction")]
pub fn consume_ready_launch_context() -> Option<LaunchContext> {
    HANDOFF_CONSUME_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    let Some(index) = registry
        .iter()
        .position(|entry| entry.stage == LaunchStage::Ready)
    else {
        HANDOFF_CONSUME_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    };

    let entry = registry.remove(index);
    HANDOFF_CONSUME_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Some(build_context(
        entry.process_id,
        &entry.process,
        entry.task_id,
    ))
}

#[cfg(feature = "process_abstraction")]
pub fn execute_ready_launch_context_on_current_cpu() -> Option<LaunchContext> {
    HANDOFF_EXECUTE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let candidate = {
        let mut registry = PROCESS_REGISTRY.lock();
        recycle_stale_handoffs(&mut registry, now_epoch);
        let Some(entry) = registry
            .iter_mut()
            .find(|entry| entry.stage == LaunchStage::Ready)
        else {
            HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        entry.stage_epoch = now_epoch;
        build_context(entry.process_id, &entry.process, entry.task_id)
    };

    let Some(cpu) = (unsafe { CpuLocal::try_get() }) else {
        HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    };

    let task_found = {
        let mut scheduler = cpu.scheduler.lock();
        match scheduler.get_task_mut(candidate.task_id) {
            Some(task_arc) => {
                let mut task = task_arc.lock();
                task.state = TaskState::Running;
                if let Some(process) = query::process_arc_by_id(candidate.process_id) {
                    process.mark_running();
                }

                #[cfg(feature = "ring_protection")]
                cpu.kernel_stack_top
                    .store(task.kernel_stack_pointer as usize, Ordering::Relaxed);

                true
            }
            None => false,
        }
    };

    if !task_found {
        HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    }

    cpu.current_task
        .store(candidate.task_id.0, Ordering::Relaxed);
    crate::kernel::rt_preemption::request_forced_reschedule();

    {
        let mut registry = PROCESS_REGISTRY.lock();
        if let Some(index) = registry.iter().position(|entry| {
            entry.process_id == candidate.process_id
                && entry.task_id == candidate.task_id
                && entry.stage == LaunchStage::Ready
        }) {
            registry.remove(index);
        }
    }

    HANDOFF_EXECUTE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Some(candidate)
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_process_with_status(process_id: ProcessId, status: i32) -> bool {
    TERMINATE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let (task_id, process_arc) = {
        let mut registry = PROCESS_REGISTRY.lock();
        let Some(index) = registry
            .iter()
            .position(|entry| entry.process_id == process_id)
        else {
            TERMINATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return false;
        };
        let entry = registry.remove(index);
        (entry.task_id, entry.process)
    };

    let shared_object_fini =
        crate::kernel::dynamic_linker::api::drain_pending_shared_object_fini_reports_for_process(
            process_id,
        );
    if !shared_object_fini.is_empty() {
        let pending_calls = shared_object_fini
            .iter()
            .map(|report| report.fini_calls.len())
            .sum::<usize>();
        for report in &shared_object_fini {
            process_arc.append_deferred_fini_calls(&report.fini_calls);
        }
        klog_info!(
            "process exit: pid={} deferred_shared_object_fini_reports={} total_calls={}",
            process_id.0,
            shared_object_fini.len(),
            pending_calls,
        );
    }

    let runtime_contract = process_arc.runtime_contract_snapshot();
    if !runtime_contract.fini_calls.is_empty() {
        let mut fini_preview = alloc::string::String::new();
        for (idx, addr) in runtime_contract.fini_calls.iter().take(4).enumerate() {
            if idx != 0 {
                fini_preview.push(',');
            }
            let _ = core::fmt::Write::write_fmt(&mut fini_preview, format_args!("{:#x}", addr));
        }
        klog_info!(
            "process exit: pid={} exec='{}' status={} pending_fini_hooks={} fini_preview=[{}] vdso={:#x}",
            process_id.0,
            runtime_contract.exec_path.as_str(),
            status,
            runtime_contract.fini_calls.len(),
            fini_preview.as_str(),
            runtime_contract.vdso_base,
        );
    }
    if runtime_contract.runtime_fini_entry != 0 {
        RUNTIME_FINI_TRAMPOLINES_SEEN.fetch_add(1, Ordering::Relaxed);
        RUNTIME_FINI_EXECUTION_DEFERRED.fetch_add(1, Ordering::Relaxed);
        klog_info!(
            "process exit: pid={} exec='{}' runtime_fini_entry={:#x} pending_fini_hooks={} execution=deferred",
            process_id.0,
            runtime_contract.exec_path.as_str(),
            runtime_contract.runtime_fini_entry,
            runtime_contract.fini_calls.len(),
        );
    }

    process_arc.mark_exited(status);
    process_arc.clear_runtime_contract();
    wrappers::finalize_task_user_exit_state(task_id);

    let cpus = crate::hal::smp::CPUS.lock();
    for cpu in cpus.iter() {
        let mut scheduler = cpu.scheduler.lock();
        scheduler.remove_task(task_id);
    }

    TERMINATE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_task(task_id: TaskId) -> bool {
    TERMINATE_BY_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let process_id = {
        let registry = PROCESS_REGISTRY.lock();
        registry
            .iter()
            .find(|entry| entry.task_id == task_id)
            .map(|entry| entry.process_id)
    };

    let Some(process_id) = process_id else {
        TERMINATE_BY_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
        return false;
    };

    if terminate_process_with_status(process_id, 0) {
        TERMINATE_BY_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
        true
    } else {
        TERMINATE_BY_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
        false
    }
}
