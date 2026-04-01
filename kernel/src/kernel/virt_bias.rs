use crate::hal::common::virt::{
    current_virtualization_runtime_governor, GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED,
};

const EIGHTH_DIVISOR: u64 = 8;
const QUARTER_DIVISOR: usize = 4;

#[inline(always)]
pub fn current_latency_bias() -> &'static str {
    current_virtualization_runtime_governor().latency_bias
}

#[inline(always)]
pub fn adjust_budget_u64(budget: u64, latency_bias: &'static str) -> u64 {
    if budget == 0 {
        return 0;
    }

    let adjustment = (budget / EIGHTH_DIVISOR).max(1);
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => budget.saturating_add(adjustment),
        GOVERNOR_BIAS_RELAXED => budget.saturating_sub(adjustment).max(1),
        _ => budget,
    }
}

#[inline(always)]
pub fn adjust_inverse_budget_u64(budget: u64, latency_bias: &'static str) -> u64 {
    if budget == 0 {
        return 0;
    }

    let adjustment = (budget / EIGHTH_DIVISOR).max(1);
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => budget.saturating_sub(adjustment).max(1),
        GOVERNOR_BIAS_RELAXED => budget.saturating_add(adjustment),
        _ => budget,
    }
}

#[inline(always)]
pub fn adjust_budget_u32(budget: u32, latency_bias: &'static str) -> u32 {
    let budget = budget.max(1);
    let adjustment = (budget / QUARTER_DIVISOR as u32).max(1);
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => budget.saturating_add(adjustment),
        GOVERNOR_BIAS_RELAXED => budget.saturating_sub(adjustment).max(1),
        _ => budget,
    }
}

#[inline(always)]
pub fn adjust_budget_usize(budget: usize, latency_bias: &'static str) -> usize {
    let budget = budget.max(1);
    let adjustment = (budget / QUARTER_DIVISOR).max(1);
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => budget.saturating_add(adjustment),
        GOVERNOR_BIAS_RELAXED => budget.saturating_sub(adjustment).max(1),
        _ => budget,
    }
}

#[inline(always)]
pub fn adjust_pct_u8(pct: u8, latency_bias: &'static str, adjustment: u8) -> u8 {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => pct.saturating_sub(adjustment).max(1),
        GOVERNOR_BIAS_RELAXED => pct.saturating_add(adjustment).min(99),
        _ => pct,
    }
}
