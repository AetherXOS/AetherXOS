use core::sync::atomic::{AtomicU64, Ordering};
use crate::kernel::debug_trace::{TraceCategory, TraceSeverity};
use crate::config::KernelConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RebalanceDecisionReason {
    Unknown = 0,
    InsufficientCpus = 1,
    NoCandidates = 2,
    SameCpu = 3,
    BelowThreshold = 4,
    NoEligibleTasks = 5,
    Rebalanced = 6,
}

impl RebalanceDecisionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RebalanceDecisionReason::Unknown => "unknown",
            RebalanceDecisionReason::InsufficientCpus => "insufficient_cpus",
            RebalanceDecisionReason::NoCandidates => "no_candidates",
            RebalanceDecisionReason::SameCpu => "same_cpu",
            RebalanceDecisionReason::BelowThreshold => "below_threshold",
            RebalanceDecisionReason::NoEligibleTasks => "no_eligible_tasks",
            RebalanceDecisionReason::Rebalanced => "rebalanced",
        }
    }

    pub(crate) fn from_raw(raw: u64) -> Self {
        match raw {
            1 => RebalanceDecisionReason::InsufficientCpus,
            2 => RebalanceDecisionReason::NoCandidates,
            3 => RebalanceDecisionReason::SameCpu,
            4 => RebalanceDecisionReason::BelowThreshold,
            5 => RebalanceDecisionReason::NoEligibleTasks,
            6 => RebalanceDecisionReason::Rebalanced,
            _ => RebalanceDecisionReason::Unknown,
        }
    }
}

pub(crate) static REBALANCE_TRACE_EVENT_SEQ: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_REASON: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_SOURCE_LOAD: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_TARGET_LOAD: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_IMBALANCE: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_THRESHOLD: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_BATCH: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_LAST_MOVED: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub(crate) fn should_emit_rebalance_trace() -> bool {
    let seq = REBALANCE_TRACE_EVENT_SEQ
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    KernelConfig::should_emit_scheduler_trace_sample(seq)
}

#[inline(always)]
pub(crate) fn record_rebalance_decision(
    reason: RebalanceDecisionReason,
    source_load: usize,
    target_load: usize,
    imbalance: usize,
    threshold: usize,
    batch: usize,
    moved: usize,
) {
    REBALANCE_LAST_REASON.store(reason as u64, Ordering::Relaxed);
    REBALANCE_LAST_SOURCE_LOAD.store(source_load as u64, Ordering::Relaxed);
    REBALANCE_LAST_TARGET_LOAD.store(target_load as u64, Ordering::Relaxed);
    REBALANCE_LAST_IMBALANCE.store(imbalance as u64, Ordering::Relaxed);
    REBALANCE_LAST_THRESHOLD.store(threshold as u64, Ordering::Relaxed);
    REBALANCE_LAST_BATCH.store(batch as u64, Ordering::Relaxed);
    REBALANCE_LAST_MOVED.store(moved as u64, Ordering::Relaxed);

    if should_emit_rebalance_trace() {
        crate::kernel::debug_trace::record_with_metadata(
            "scheduler",
            reason.as_str(),
            Some(reason as u64),
            false,
            TraceSeverity::Trace,
            TraceCategory::Scheduler,
        );
    }
}
