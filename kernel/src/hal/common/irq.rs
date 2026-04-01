#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StormDecision {
    pub in_storm: bool,
    pub should_log: bool,
    pub first_storm_event: bool,
    pub suppressed_log: bool,
}

#[inline(always)]
pub fn tracked_limit(configured_limit: usize, max_tracked: usize) -> usize {
    configured_limit.min(max_tracked)
}

#[inline(always)]
pub fn reset_window(start: u64, now: u64, window_ticks: u64) -> bool {
    start == 0 || now.wrapping_sub(start) >= window_ticks.max(1)
}

#[inline(always)]
pub fn storm_decision(
    event_count: u64,
    threshold: u64,
    log_every: u64,
    log_when_healthy: bool,
) -> StormDecision {
    let threshold = threshold.max(1);
    let log_every = log_every.max(1);
    let in_storm = event_count > threshold;
    if !in_storm {
        return StormDecision {
            in_storm,
            should_log: log_when_healthy,
            first_storm_event: false,
            suppressed_log: false,
        };
    }
    if event_count == threshold.saturating_add(1) {
        return StormDecision {
            in_storm,
            should_log: true,
            first_storm_event: true,
            suppressed_log: false,
        };
    }
    if (event_count - threshold) % log_every == 0 {
        StormDecision {
            in_storm,
            should_log: true,
            first_storm_event: false,
            suppressed_log: false,
        }
    } else {
        StormDecision {
            in_storm,
            should_log: false,
            first_storm_event: false,
            suppressed_log: true,
        }
    }
}

#[inline(always)]
pub fn hottest_counter_index(counters: &[u64]) -> usize {
    let mut best_idx = 0usize;
    let mut best_total = 0u64;
    for (idx, total) in counters.iter().copied().enumerate() {
        if total > best_total {
            best_total = total;
            best_idx = idx;
        }
    }
    best_idx
}

#[inline(always)]
pub fn abs_diff_u64(a: u64, b: u64) -> u64 {
    a.abs_diff(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn storm_decision_handles_healthy_first_storm_and_suppressed_cases() {
        assert_eq!(
            storm_decision(1, 4, 3, true),
            StormDecision {
                in_storm: false,
                should_log: true,
                first_storm_event: false,
                suppressed_log: false,
            }
        );
        assert_eq!(
            storm_decision(5, 4, 3, false),
            StormDecision {
                in_storm: true,
                should_log: true,
                first_storm_event: true,
                suppressed_log: false,
            }
        );
        assert_eq!(
            storm_decision(6, 4, 3, false),
            StormDecision {
                in_storm: true,
                should_log: false,
                first_storm_event: false,
                suppressed_log: true,
            }
        );
    }

    #[test_case]
    fn hottest_counter_index_picks_largest_entry() {
        assert_eq!(hottest_counter_index(&[1, 7, 3]), 1);
        assert_eq!(hottest_counter_index(&[0, 0, 0]), 0);
    }

    #[test_case]
    fn reset_window_and_abs_diff_behave_as_expected() {
        assert!(reset_window(0, 10, 50));
        assert!(!reset_window(10, 30, 50));
        assert!(reset_window(10, 70, 50));
        assert_eq!(abs_diff_u64(10, 4), 6);
        assert_eq!(abs_diff_u64(4, 10), 6);
    }
}
