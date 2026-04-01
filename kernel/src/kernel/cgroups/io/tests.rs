use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn io_limit_bias_adjustment_tracks_latency_profile() {
    let limit = IoDeviceLimit {
        rbps: 80,
        wbps: 80,
        riops: 8,
        wiops: 8,
    };

    let balanced = io_limit_with_bias(limit, GOVERNOR_BIAS_BALANCED);
    let aggressive = io_limit_with_bias(limit, GOVERNOR_BIAS_AGGRESSIVE);
    let relaxed = io_limit_with_bias(limit, GOVERNOR_BIAS_RELAXED);

    assert_eq!(balanced.rbps, 80);
    assert_eq!(aggressive.rbps, 90);
    assert_eq!(relaxed.rbps, 70);
    assert_eq!(aggressive.riops, 9);
    assert_eq!(relaxed.riops, 7);
}

#[test_case]
fn unlimited_io_limits_remain_unlimited() {
    let limit = IoDeviceLimit::unlimited();
    assert_eq!(io_limit_with_bias(limit, GOVERNOR_BIAS_BALANCED).rbps, 0);
    assert_eq!(io_limit_with_bias(limit, GOVERNOR_BIAS_AGGRESSIVE).wbps, 0);
    assert_eq!(io_limit_with_bias(limit, GOVERNOR_BIAS_RELAXED).riops, 0);
    assert_eq!(io_limit_with_bias(limit, GOVERNOR_BIAS_RELAXED).wiops, 0);
}

#[test_case]
fn io_bias_helpers_shift_small_limits_in_both_directions() {
    let limit = IoDeviceLimit {
        rbps: 8,
        wbps: 8,
        riops: 1,
        wiops: 1,
    };

    let aggressive = io_limit_with_bias(limit, GOVERNOR_BIAS_AGGRESSIVE);
    let relaxed = io_limit_with_bias(limit, GOVERNOR_BIAS_RELAXED);

    assert!(aggressive.rbps > limit.rbps);
    assert!(aggressive.wbps > limit.wbps);
    assert!(relaxed.rbps < limit.rbps);
    assert!(relaxed.wbps < limit.wbps);
    assert_eq!(aggressive.riops, 2);
    assert_eq!(relaxed.riops, 1);
}
