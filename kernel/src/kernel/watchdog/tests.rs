#[cfg(target_os = "none")]
#[test_case]
fn governor_adjusted_hard_stall_ticks_tracks_latency_bias() {
    assert_eq!(governor_adjusted_hard_stall_ticks(8, "balanced"), 8);
    assert_eq!(governor_adjusted_hard_stall_ticks(8, "aggressive"), 6);
    assert_eq!(governor_adjusted_hard_stall_ticks(8, "relaxed"), 10);
    assert_eq!(governor_adjusted_hard_stall_ticks(1, "aggressive"), 1);
}
