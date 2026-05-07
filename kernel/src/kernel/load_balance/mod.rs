pub mod adaptive;
pub mod decision;
pub mod operations;
pub mod stats;

pub(crate) use stats::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn reset_rebalance_adaptive_state() {
    use core::sync::atomic::Ordering;
    use crate::kernel::load_balance::adaptive::REBALANCE_IMBALANCE_RING;
    use crate::kernel::load_balance::adaptive::REBALANCE_IMBALANCE_SEQ;
    use crate::kernel::load_balance::decision::RebalanceDecisionReason;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_BATCH;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_IMBALANCE;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_MOVED;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_REASON;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_SOURCE_LOAD;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_TARGET_LOAD;
    use crate::kernel::load_balance::decision::REBALANCE_LAST_THRESHOLD;
    use crate::kernel::load_balance::stats::REBALANCE_IMBALANCE_BIN_2_3;
    use crate::kernel::load_balance::stats::REBALANCE_IMBALANCE_BIN_4_7;
    use crate::kernel::load_balance::stats::REBALANCE_IMBALANCE_BIN_8_15;
    use crate::kernel::load_balance::stats::REBALANCE_IMBALANCE_BIN_GE16;
    use crate::kernel::load_balance::stats::REBALANCE_IMBALANCE_BIN_LT2;

    REBALANCE_IMBALANCE_SEQ.store(0, Ordering::Relaxed);
    REBALANCE_IMBALANCE_BIN_LT2.store(0, Ordering::Relaxed);
    REBALANCE_IMBALANCE_BIN_2_3.store(0, Ordering::Relaxed);
    REBALANCE_IMBALANCE_BIN_4_7.store(0, Ordering::Relaxed);
    REBALANCE_IMBALANCE_BIN_8_15.store(0, Ordering::Relaxed);
    REBALANCE_IMBALANCE_BIN_GE16.store(0, Ordering::Relaxed);
    let mut ring = REBALANCE_IMBALANCE_RING.lock();
    for value in ring.iter_mut() {
        *value = 0;
    }
    REBALANCE_LAST_REASON.store(RebalanceDecisionReason::Unknown as u64, Ordering::Relaxed);
    REBALANCE_LAST_SOURCE_LOAD.store(0, Ordering::Relaxed);
    REBALANCE_LAST_TARGET_LOAD.store(0, Ordering::Relaxed);
    REBALANCE_LAST_IMBALANCE.store(0, Ordering::Relaxed);
    REBALANCE_LAST_THRESHOLD.store(0, Ordering::Relaxed);
    REBALANCE_LAST_BATCH.store(0, Ordering::Relaxed);
    REBALANCE_LAST_MOVED.store(0, Ordering::Relaxed);
}
