use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::interfaces::task::{ProcessId, TaskId};
use crate::interfaces::Scheduler;

#[cfg(feature = "process_abstraction")]
use crate::kernel::process::Process;

pub mod state;
pub mod types;

pub use state::*;
pub use types::*;

#[cfg(feature = "process_abstraction")]
pub mod process_runtime;

#[cfg(feature = "process_abstraction")]
pub use process_runtime::*;

#[cfg(test)]
mod tests;

pub fn stats() -> LaunchStats {
    LaunchStats {
        spawn_attempts: SPAWN_ATTEMPTS.load(Ordering::Relaxed),
        spawn_success: SPAWN_SUCCESS.load(Ordering::Relaxed),
        spawn_failures: SPAWN_FAILURES.load(Ordering::Relaxed),
        enqueue_failures: ENQUEUE_FAILURES.load(Ordering::Relaxed),
        validation_failures: VALIDATION_FAILURES.load(Ordering::Relaxed),
        terminate_attempts: TERMINATE_ATTEMPTS.load(Ordering::Relaxed),
        terminate_success: TERMINATE_SUCCESS.load(Ordering::Relaxed),
        terminate_failures: TERMINATE_FAILURES.load(Ordering::Relaxed),
        claim_attempts: CLAIM_ATTEMPTS.load(Ordering::Relaxed),
        claim_success: CLAIM_SUCCESS.load(Ordering::Relaxed),
        claim_failures: CLAIM_FAILURES.load(Ordering::Relaxed),
        handoff_ack_attempts: HANDOFF_ACK_ATTEMPTS.load(Ordering::Relaxed),
        handoff_ack_success: HANDOFF_ACK_SUCCESS.load(Ordering::Relaxed),
        handoff_ack_failures: HANDOFF_ACK_FAILURES.load(Ordering::Relaxed),
        handoff_consume_attempts: HANDOFF_CONSUME_ATTEMPTS.load(Ordering::Relaxed),
        handoff_consume_success: HANDOFF_CONSUME_SUCCESS.load(Ordering::Relaxed),
        handoff_consume_failures: HANDOFF_CONSUME_FAILURES.load(Ordering::Relaxed),
        handoff_execute_attempts: HANDOFF_EXECUTE_ATTEMPTS.load(Ordering::Relaxed),
        handoff_execute_success: HANDOFF_EXECUTE_SUCCESS.load(Ordering::Relaxed),
        handoff_execute_failures: HANDOFF_EXECUTE_FAILURES.load(Ordering::Relaxed),
        terminate_by_task_attempts: TERMINATE_BY_TASK_ATTEMPTS.load(Ordering::Relaxed),
        terminate_by_task_success: TERMINATE_BY_TASK_SUCCESS.load(Ordering::Relaxed),
        terminate_by_task_failures: TERMINATE_BY_TASK_FAILURES.load(Ordering::Relaxed),
        stale_scan_calls: STALE_SCAN_CALLS.load(Ordering::Relaxed),
        stale_recycled_entries: STALE_RECYCLED_ENTRIES.load(Ordering::Relaxed),
        stale_claim_timeouts: STALE_CLAIM_TIMEOUTS.load(Ordering::Relaxed),
        stale_ready_timeouts: STALE_READY_TIMEOUTS.load(Ordering::Relaxed),
        runtime_fini_trampolines_seen: RUNTIME_FINI_TRAMPOLINES_SEEN.load(Ordering::Relaxed),
        runtime_fini_execution_deferred: RUNTIME_FINI_EXECUTION_DEFERRED.load(Ordering::Relaxed),
        registered_processes: {
            #[cfg(feature = "process_abstraction")]
            {
                PROCESS_REGISTRY.lock().len()
            }
            #[cfg(not(feature = "process_abstraction"))]
            {
                0
            }
        },
        last_task_id: TaskId(LAST_TASK_ID.load(Ordering::Relaxed)),
    }
}
