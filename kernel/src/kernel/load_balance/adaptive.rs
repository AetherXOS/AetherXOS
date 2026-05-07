use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use crate::config::KernelConfig;
use super::stats::{
    REBALANCE_IMBALANCE_BIN_2_3, REBALANCE_IMBALANCE_BIN_4_7, REBALANCE_IMBALANCE_BIN_8_15,
    REBALANCE_IMBALANCE_BIN_GE16, REBALANCE_IMBALANCE_BIN_LT2,
};

pub(crate) static REBALANCE_IMBALANCE_SEQ: AtomicU64 = AtomicU64::new(0);
pub(crate) const IMBALANCE_WINDOW: usize = crate::generated_consts::GOVERNOR_LOAD_BALANCE_PERCENTILE_WINDOW;
pub(crate) static REBALANCE_IMBALANCE_RING: Mutex<[u32; IMBALANCE_WINDOW]> = Mutex::new([0; IMBALANCE_WINDOW]);
const IMBALANCE_DECAY_DENOM: usize = 8;
const IMBALANCE_DECAY_OLDEST_NUM: usize = 4;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RebalancePercentiles {
    pub(crate) samples: usize,
    pub(crate) p50: usize,
    pub(crate) p90: usize,
    pub(crate) p99: usize,
}

#[inline(always)]
fn percentile_index(n: usize, pct: usize) -> usize {
    if n <= 1 {
        0
    } else {
        ((n - 1) * pct) / 100
    }
}

pub(crate) fn imbalance_percentiles_snapshot() -> RebalancePercentiles {
    let seq = REBALANCE_IMBALANCE_SEQ.load(Ordering::Relaxed) as usize;
    let total = core::cmp::min(seq, IMBALANCE_WINDOW);
    if total == 0 {
        return RebalancePercentiles {
            samples: 0,
            p50: 0,
            p90: 0,
            p99: 0,
        };
    }

    let oldest = if total == IMBALANCE_WINDOW {
        seq % IMBALANCE_WINDOW
    } else {
        0
    };
    let ring = REBALANCE_IMBALANCE_RING.lock();
    let mut samples = Vec::with_capacity(total);
    let mut cursor = oldest;
    let decay_span = total.saturating_sub(1).max(1);
    for age_index in 0..total {
        let raw = ring[cursor] as usize;
        let decay_num = IMBALANCE_DECAY_OLDEST_NUM
            + ((IMBALANCE_DECAY_DENOM - IMBALANCE_DECAY_OLDEST_NUM) * age_index) / decay_span;
        samples.push(raw.saturating_mul(decay_num) / IMBALANCE_DECAY_DENOM);
        cursor = (cursor + 1) % IMBALANCE_WINDOW;
    }
    drop(ring);
    samples.sort_unstable();

    let p50 = samples[percentile_index(samples.len(), 50)];
    let p90 = samples[percentile_index(samples.len(), 90)];
    let p99 = samples[percentile_index(samples.len(), 99)];
    RebalancePercentiles {
        samples: samples.len(),
        p50,
        p90,
        p99,
    }
}

#[inline(always)]
pub(crate) fn record_imbalance_histogram(imbalance: usize) {
    let seq = REBALANCE_IMBALANCE_SEQ
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let idx = (seq as usize) % IMBALANCE_WINDOW;
    let value = core::cmp::min(imbalance, u32::MAX as usize) as u32;
    REBALANCE_IMBALANCE_RING.lock()[idx] = value;

    match imbalance {
        0..=1 => {
            REBALANCE_IMBALANCE_BIN_LT2.fetch_add(1, Ordering::Relaxed);
        }
        2..=3 => {
            REBALANCE_IMBALANCE_BIN_2_3.fetch_add(1, Ordering::Relaxed);
        }
        4..=7 => {
            REBALANCE_IMBALANCE_BIN_4_7.fetch_add(1, Ordering::Relaxed);
        }
        8..=15 => {
            REBALANCE_IMBALANCE_BIN_8_15.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            REBALANCE_IMBALANCE_BIN_GE16.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[inline(always)]
pub(crate) fn rebalance_threshold(tuning: crate::hal::common::virt::VirtualizationRebalanceTuning) -> usize {
    let base = KernelConfig::rebalance_imbalance_threshold()
        .saturating_div(tuning.threshold_divisor.max(1))
        .max(1);
    let p = imbalance_percentiles_snapshot();
    if p.samples < 8 {
        return base;
    }

    let tail = ((p.p90 + p.p99) / 2).max(1);
    let adaptive_floor = tail
        .saturating_div(tuning.threshold_divisor.max(1))
        .max(1);
    base.max(adaptive_floor)
}

#[inline(always)]
pub(crate) fn rebalance_batch_size(tuning: crate::hal::common::virt::VirtualizationRebalanceTuning) -> usize {
    let base = KernelConfig::rebalance_batch_size()
        .saturating_mul(tuning.batch_multiplier.max(1))
        .max(1);
    let p = imbalance_percentiles_snapshot();
    if p.samples < 8 {
        return base;
    }

    let spread = p.p99.saturating_sub(p.p50);
    let bonus = (spread / 8).saturating_mul(tuning.batch_multiplier.max(1));
    base.saturating_add(bonus).max(1)
}

#[inline(always)]
pub(crate) fn prefer_local_skip_budget(
    tuning: crate::hal::common::virt::VirtualizationRebalanceTuning,
) -> usize {
    let base = KernelConfig::rebalance_prefer_local_skip_budget()
        .saturating_div(tuning.prefer_local_skip_budget_divisor.max(1))
        .max(1);
    let p = imbalance_percentiles_snapshot();
    if p.samples < 8 {
        return base;
    }

    let tail_pressure = p.p90.saturating_sub(p.p50);
    let adaptive_add = (tail_pressure / 8).min(4);
    base.saturating_add(adaptive_add)
}
