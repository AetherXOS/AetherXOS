use super::super::*;
use super::super::support::*;

use core::sync::atomic::Ordering;
use crate::observability_launch;

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
        observability_launch! {
            crate::kernel::debug_trace::record_optional(
                "launch.handoff",
                "consume_empty",
                Some(now_epoch),
                false,
            );
        }
        return None;
    };

    let entry = registry.remove(index);
    Some(lifecycle_helpers::consume_entry_and_build(entry, now_epoch))
}
