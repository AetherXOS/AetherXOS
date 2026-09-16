use super::*;
use core::sync::atomic::Ordering;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub(super) fn claim_mark_claimed_and_build(entry: &mut LaunchRegistryEntry, now_epoch: u64) -> LaunchContext {
    entry.stage = LaunchStage::Claimed;
    entry.stage_epoch = now_epoch;
    CLAIM_SUCCESS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.handoff",
            "claim_pending",
            Some(entry.process_id.0 as u64),
            false,
        );
        crate::klog_info!(
            "launch handoff claim: pid={} tid={} stage={:?} epoch={}",
            entry.process_id.0,
            entry.task_id.0,
            entry.stage,
            now_epoch,
        );
    }
    support::build_context(entry.process_id, &entry.process, entry.task_id)
}

#[cfg(feature = "process_abstraction")]
pub(super) fn ack_set_stage_and_log(entry: &mut LaunchRegistryEntry, success: bool, now_epoch: u64) {
    entry.stage = if success { LaunchStage::Ready } else { LaunchStage::Pending };
    entry.stage_epoch = now_epoch;
    HANDOFF_ACK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.handoff",
            if success { "ack_success" } else { "ack_retry" },
            Some(entry.process_id.0 as u64),
            false,
        );
        crate::klog_info!(
            "launch handoff ack: pid={} success={} new_stage={:?} epoch={}",
            entry.process_id.0,
            success,
            entry.stage,
            now_epoch,
        );
    }
}

#[cfg(feature = "process_abstraction")]
pub(super) fn consume_entry_and_build(entry: LaunchRegistryEntry, _now_epoch: u64) -> LaunchContext {
    HANDOFF_CONSUME_SUCCESS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::klog_info!(
            "launch handoff consume: pid={} tid={} stage={:?} epoch= {}",
            entry.process_id.0,
            entry.task_id.0,
            entry.stage,
            now_epoch,
        );
    }
    support::build_context(entry.process_id, &entry.process, entry.task_id)
}

#[cfg(feature = "process_abstraction")]
pub(super) fn exec_prepare_candidate(entry: &mut LaunchRegistryEntry, now_epoch: u64) -> LaunchContext {
    entry.stage_epoch = now_epoch;
    observability_launch! {
        crate::klog_info!(
            "launch handoff execute candidate: pid={} tid={} epoch={} ",
            entry.process_id.0,
            entry.task_id.0,
            now_epoch,
        );
    }
    support::build_context(entry.process_id, &entry.process, entry.task_id)
}
