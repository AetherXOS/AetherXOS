use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn memory_limit_bias_adjustment_tracks_latency_profile() {
    assert_eq!(
        governor_adjusted_memory_budget(80, GOVERNOR_BIAS_BALANCED),
        80
    );
    assert_eq!(
        governor_adjusted_memory_budget(80, GOVERNOR_BIAS_AGGRESSIVE),
        90
    );
    assert_eq!(
        governor_adjusted_memory_budget(80, GOVERNOR_BIAS_RELAXED),
        70
    );
}

#[test_case]
fn memory_protection_bias_moves_in_inverse_direction() {
    assert_eq!(
        governor_adjusted_memory_protection(80, GOVERNOR_BIAS_BALANCED),
        80
    );
    assert_eq!(
        governor_adjusted_memory_protection(80, GOVERNOR_BIAS_AGGRESSIVE),
        70
    );
    assert_eq!(
        governor_adjusted_memory_protection(80, GOVERNOR_BIAS_RELAXED),
        90
    );
}

#[test_case]
fn unlimited_memory_limits_remain_unlimited() {
    assert_eq!(
        governor_adjusted_memory_budget(0, GOVERNOR_BIAS_BALANCED),
        0
    );
    assert_eq!(
        governor_adjusted_memory_budget(0, GOVERNOR_BIAS_AGGRESSIVE),
        0
    );
    assert_eq!(
        governor_adjusted_memory_protection(0, GOVERNOR_BIAS_RELAXED),
        0
    );
}

#[test_case]
fn memory_bias_helpers_shift_small_limits_in_both_directions() {
    assert!(governor_adjusted_memory_budget(8, GOVERNOR_BIAS_AGGRESSIVE) > 8);
    assert!(governor_adjusted_memory_budget(8, GOVERNOR_BIAS_RELAXED) < 8);
    assert!(governor_adjusted_memory_protection(8, GOVERNOR_BIAS_AGGRESSIVE) < 8);
    assert!(governor_adjusted_memory_protection(8, GOVERNOR_BIAS_RELAXED) > 8);
}
