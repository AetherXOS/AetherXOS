use super::super::*;
use super::super::support::*;

use core::sync::atomic::Ordering;
use crate::interfaces::task::ProcessId;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub fn claim_next_launch_context() -> Option<LaunchContext> {
    CLAIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    for entry in registry.iter_mut() {
        if entry.stage == LaunchStage::Pending {
            return Some(lifecycle_helpers::claim_mark_claimed_and_build(entry, now_epoch));
        }
    }

    CLAIM_FAILURES.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.handoff",
            "claim_empty",
            Some(now_epoch),
            false,
        );
    }
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
        crate::klog_warn!(
            "launch handoff ack missing: pid={} success={} epoch= {}",
            process_id.0,
            success,
            now_epoch,
        );
        return false;
    };

    entry.stage = if success {
        LaunchStage::Ready
    } else {
        LaunchStage::Pending
    };
    entry.stage_epoch = now_epoch;
    HANDOFF_ACK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.handoff",
            if success { "ack_success" } else { "ack_retry" },
            Some(process_id.0 as u64),
            false,
        );
        crate::klog_info!(
            "launch handoff ack: pid={} success={} new_stage={:?} epoch={}",
            process_id.0,
            success,
            entry.stage,
            now_epoch,
        );
    }
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
