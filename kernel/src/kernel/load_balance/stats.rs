use core::sync::atomic::{AtomicU64, Ordering};
use super::decision::{
    RebalanceDecisionReason, REBALANCE_LAST_BATCH, REBALANCE_LAST_IMBALANCE,
    REBALANCE_LAST_MOVED, REBALANCE_LAST_REASON, REBALANCE_LAST_SOURCE_LOAD,
    REBALANCE_LAST_TARGET_LOAD, REBALANCE_LAST_THRESHOLD,
};
use super::adaptive::imbalance_percentiles_snapshot;

pub(crate) static GLOBAL_TICK: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_MOVED: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_AFFINITY_SKIPS: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_PREFER_LOCAL_SKIPS: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_PREFER_LOCAL_FORCED_MOVES: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_IMBALANCE_BIN_LT2: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_IMBALANCE_BIN_2_3: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_IMBALANCE_BIN_4_7: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_IMBALANCE_BIN_8_15: AtomicU64 = AtomicU64::new(0);
pub(crate) static REBALANCE_IMBALANCE_BIN_GE16: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct RebalanceStats {
    pub attempts: u64,
    pub moved: u64,
    pub affinity_skips: u64,
    pub prefer_local_skips: u64,
    pub prefer_local_forced_moves: u64,
    pub imbalance_lt2: u64,
    pub imbalance_2_3: u64,
    pub imbalance_4_7: u64,
    pub imbalance_8_15: u64,
    pub imbalance_ge16: u64,
    pub imbalance_samples: usize,
    pub imbalance_p50: usize,
    pub imbalance_p90: usize,
    pub imbalance_p99: usize,
    pub last_reason: RebalanceDecisionReason,
    pub last_source_load: usize,
    pub last_target_load: usize,
    pub last_imbalance: usize,
    pub last_threshold: usize,
    pub last_batch: usize,
    pub last_moved: usize,
}

#[inline(always)]
pub fn stats_snapshot() -> RebalanceStats {
    let p = imbalance_percentiles_snapshot();
    RebalanceStats {
        attempts: REBALANCE_ATTEMPTS.load(Ordering::Relaxed),
        moved: REBALANCE_MOVED.load(Ordering::Relaxed),
        affinity_skips: REBALANCE_AFFINITY_SKIPS.load(Ordering::Relaxed),
        prefer_local_skips: REBALANCE_PREFER_LOCAL_SKIPS.load(Ordering::Relaxed),
        prefer_local_forced_moves: REBALANCE_PREFER_LOCAL_FORCED_MOVES.load(Ordering::Relaxed),
        imbalance_lt2: REBALANCE_IMBALANCE_BIN_LT2.load(Ordering::Relaxed),
        imbalance_2_3: REBALANCE_IMBALANCE_BIN_2_3.load(Ordering::Relaxed),
        imbalance_4_7: REBALANCE_IMBALANCE_BIN_4_7.load(Ordering::Relaxed),
        imbalance_8_15: REBALANCE_IMBALANCE_BIN_8_15.load(Ordering::Relaxed),
        imbalance_ge16: REBALANCE_IMBALANCE_BIN_GE16.load(Ordering::Relaxed),
        imbalance_samples: p.samples,
        imbalance_p50: p.p50,
        imbalance_p90: p.p90,
        imbalance_p99: p.p99,
        last_reason: RebalanceDecisionReason::from_raw(REBALANCE_LAST_REASON.load(Ordering::Relaxed)),
        last_source_load: REBALANCE_LAST_SOURCE_LOAD.load(Ordering::Relaxed) as usize,
        last_target_load: REBALANCE_LAST_TARGET_LOAD.load(Ordering::Relaxed) as usize,
        last_imbalance: REBALANCE_LAST_IMBALANCE.load(Ordering::Relaxed) as usize,
        last_threshold: REBALANCE_LAST_THRESHOLD.load(Ordering::Relaxed) as usize,
        last_batch: REBALANCE_LAST_BATCH.load(Ordering::Relaxed) as usize,
        last_moved: REBALANCE_LAST_MOVED.load(Ordering::Relaxed) as usize,
    }
}
