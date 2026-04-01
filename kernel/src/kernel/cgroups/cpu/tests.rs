use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn quota_adjustment_tracks_virtualization_latency_bias() {
    assert_eq!(governor_adjusted_quota_us(80, GOVERNOR_BIAS_BALANCED), 80);
    assert_eq!(governor_adjusted_quota_us(80, GOVERNOR_BIAS_AGGRESSIVE), 90);
    assert_eq!(governor_adjusted_quota_us(80, GOVERNOR_BIAS_RELAXED), 70);
}

#[test_case]
fn zero_quota_stays_unlimited_under_all_biases() {
    assert_eq!(governor_adjusted_quota_us(0, GOVERNOR_BIAS_BALANCED), 0);
    assert_eq!(governor_adjusted_quota_us(0, GOVERNOR_BIAS_AGGRESSIVE), 0);
    assert_eq!(governor_adjusted_quota_us(0, GOVERNOR_BIAS_RELAXED), 0);
}

#[test_case]
fn remaining_quota_uses_effective_quota_baseline() {
    let mut ctrl = CpuController::new();
    ctrl.quota_us = 80;
    ctrl.used_us = 10;
    assert!(ctrl.remaining_us() <= 90);
    assert!(ctrl.remaining_us() >= 1);
}
