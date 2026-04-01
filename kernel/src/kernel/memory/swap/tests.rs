use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn swap_readahead_bias_adjustment_tracks_latency_profile() {
    assert_eq!(
        governor_adjusted_swap_readahead(8, GOVERNOR_BIAS_BALANCED),
        8
    );
    assert_eq!(
        governor_adjusted_swap_readahead(8, GOVERNOR_BIAS_AGGRESSIVE),
        10
    );
    assert_eq!(
        governor_adjusted_swap_readahead(8, GOVERNOR_BIAS_RELAXED),
        6
    );
}

#[test_case]
fn swap_readahead_is_clamped_to_one() {
    assert_eq!(
        governor_adjusted_swap_readahead(0, GOVERNOR_BIAS_BALANCED),
        1
    );
    assert_eq!(
        governor_adjusted_swap_readahead(1, GOVERNOR_BIAS_RELAXED),
        1
    );
}

#[test_case]
fn set_readahead_keeps_minimum_nonzero_floor() {
    let mut manager = SwapManager::new();
    manager.set_readahead(0);
    assert!(manager.readahead() >= 1);
}
