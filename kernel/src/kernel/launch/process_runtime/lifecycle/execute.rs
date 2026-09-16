//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
use super::super::*;
use super::super::support::*;

use core::sync::atomic::Ordering;
use crate::interfaces::task::TaskState;
use crate::kernel::cpu_local::CpuLocal;
use crate::observability_launch;

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
            observability_launch! {
                crate::kernel::debug_trace::record_optional(
                    "launch.handoff",
                    "execute_empty",
                    Some(now_epoch),
                    false,
                );
            }
            return None;
        };
        lifecycle_helpers::exec_prepare_candidate(entry, now_epoch)
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
                if let Some(process) = crate::kernel::launch::process_runtime::query::process_arc_by_id(candidate.process_id) {
                    process.mark_running();
                }

                #[cfg(feature = "ring_protection")]
                cpu.kernel_stack_top
                    .store(task.kernel_stack_pointer as usize, Ordering::Relaxed);

                true
            }
            None => {
                observability_launch! {
                    crate::klog_warn!(
                        "launch handoff execute failed: pid={} tid={} reason=task-missing",
                        candidate.process_id.0,
                        candidate.task_id.0,
                    );
                }
                false
            }
        }
    };

    if !task_found {
        HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
        observability_launch! {
            crate::kernel::debug_trace::record_optional(
                "launch.handoff",
                "execute_task_missing",
                Some(candidate.process_id.0 as u64),
                false,
            );
        }
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
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.handoff",
            "execute_success",
            Some(candidate.process_id.0 as u64),
            false,
        );
    }
    Some(candidate)
}

