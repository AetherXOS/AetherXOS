use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn reclaim_config_bias_adjustment_tracks_latency_profile() {
    let base = ReclaimConfig::default();
    let balanced = reclaim_config_with_bias(base, GOVERNOR_BIAS_BALANCED);
    let aggressive = reclaim_config_with_bias(base, GOVERNOR_BIAS_AGGRESSIVE);
    let relaxed = reclaim_config_with_bias(base, GOVERNOR_BIAS_RELAXED);

    assert_eq!(balanced.high_watermark_pct, base.high_watermark_pct);
    assert!(aggressive.high_watermark_pct < base.high_watermark_pct);
    assert!(relaxed.high_watermark_pct > base.high_watermark_pct);
    assert!(aggressive.batch_size > base.batch_size);
    assert!(relaxed.batch_size < base.batch_size);
}

#[test_case]
fn watermark_adjustment_stays_bounded() {
    assert_eq!(
        governor_adjusted_watermark_pct(2, GOVERNOR_BIAS_AGGRESSIVE),
        1
    );
    assert_eq!(
        governor_adjusted_watermark_pct(98, GOVERNOR_BIAS_RELAXED),
        99
    );
}

#[test_case]
fn batch_adjustment_stays_nonzero() {
    assert_eq!(governor_adjusted_batch_size(1, GOVERNOR_BIAS_RELAXED), 1);
    assert!(governor_adjusted_batch_size(4, GOVERNOR_BIAS_AGGRESSIVE) > 4);
}
