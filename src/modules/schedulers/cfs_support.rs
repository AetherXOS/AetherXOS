pub(super) const NICE_0_LOAD: u64 = 1024;

/// Standard Linux prio_to_weight table mapped to nice values -20 to +19.
/// Nice 0 (index 20) is 1024.
pub(super) static PRIO_TO_WEIGHT: [u64; 40] = [
    88761, 71755, 56483, 46273, 36291, 29154, 23254, 18705, 14949, 11916, 9548, 7620, 6100, 4904,
    3906, 3121, 2501, 1991, 1586, 1277, 1024, 820, 655, 526, 423, 335, 272, 215, 172, 137, 110, 87,
    70, 56, 45, 36, 29, 23, 18, 15,
];

#[inline(always)]
pub(super) fn calculate_weight(priority: u8) -> u64 {
    let mut idx = (priority as usize * 40) / 256;
    if idx >= 40 {
        idx = 39;
    }
    PRIO_TO_WEIGHT[idx]
}

#[inline(always)]
pub(super) fn calc_delta_vruntime(delta_exec: u64, weight: u64) -> u64 {
    if weight == 0 {
        return delta_exec;
    }
    (delta_exec * NICE_0_LOAD) / weight
}

#[inline(always)]
pub(super) fn group_is_throttled(cpu_quota_ns: u64, cpu_used_ns: u64) -> bool {
    cpu_quota_ns > 0 && cpu_used_ns >= cpu_quota_ns
}

#[inline(always)]
pub(super) fn should_preempt_current(
    current_vruntime: u64,
    left_vruntime: u64,
    min_granularity_ns: u64,
) -> bool {
    current_vruntime > left_vruntime && current_vruntime - left_vruntime >= min_granularity_ns
}

#[inline(always)]
pub(super) fn migration_cost(
    from_core: Option<u32>,
    to_core: Option<u32>,
    from_llc: Option<u16>,
    to_llc: Option<u16>,
    from_node: Option<u8>,
    to_node: Option<u8>,
) -> u64 {
    match (from_core, to_core, from_llc, to_llc, from_node, to_node) {
        (Some(a), Some(b), _, _, _, _) if a == b => 0,
        (_, _, Some(a), Some(b), _, _) if a == b => 1,
        (_, _, _, _, Some(a), Some(b)) if a == b => 10,
        (Some(_), Some(_), Some(_), Some(_), Some(_), Some(_)) => 100,
        _ => u64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn weight_table_stays_monotonic_and_bounded() {
        assert!(calculate_weight(0) >= calculate_weight(64));
        assert!(calculate_weight(64) >= calculate_weight(128));
        assert!(calculate_weight(128) >= calculate_weight(192));
        assert!(calculate_weight(192) >= calculate_weight(255));
        assert_eq!(calculate_weight(255), 15);
    }

    #[test_case]
    fn delta_vruntime_respects_zero_and_weighted_paths() {
        assert_eq!(calc_delta_vruntime(50, 0), 50);
        assert_eq!(calc_delta_vruntime(1024, NICE_0_LOAD), 1024);
        assert!(calc_delta_vruntime(1024, NICE_0_LOAD * 2) < 1024);
    }

    #[test_case]
    fn throttle_and_preempt_helpers_follow_expected_edges() {
        assert!(!group_is_throttled(0, 999));
        assert!(!group_is_throttled(100, 99));
        assert!(group_is_throttled(100, 100));

        assert!(should_preempt_current(200, 100, 50));
        assert!(!should_preempt_current(149, 100, 50));
        assert!(!should_preempt_current(100, 100, 1));
    }

    #[test_case]
    fn migration_cost_prefers_same_llc_then_same_node() {
        assert_eq!(
            migration_cost(Some(3), Some(3), Some(1), Some(1), Some(0), Some(0)),
            0
        );
        assert_eq!(
            migration_cost(Some(1), Some(2), Some(7), Some(7), Some(0), Some(0)),
            1
        );
        assert_eq!(
            migration_cost(Some(1), Some(2), Some(7), Some(8), Some(0), Some(0)),
            10
        );
        assert_eq!(
            migration_cost(Some(1), Some(2), Some(7), Some(8), Some(0), Some(1)),
            100
        );
    }
}
