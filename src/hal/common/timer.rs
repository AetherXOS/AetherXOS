#[inline(always)]
pub fn ns_to_ticks(period_ns: u64, freq_hz: u64, fallback_divisor: u64) -> u64 {
    if freq_hz > 0 {
        period_ns.saturating_mul(freq_hz) / 1_000_000_000
    } else if fallback_divisor > 0 {
        period_ns / fallback_divisor
    } else {
        0
    }
}

#[inline(always)]
pub fn ticks_to_ns(elapsed_ticks: u64, freq_hz: u64) -> u64 {
    if freq_hz == 0 {
        0
    } else {
        elapsed_ticks.saturating_mul(1_000_000_000) / freq_hz
    }
}

#[inline(always)]
pub fn clamp_ticks(ticks: u64, min_ticks: u64, max_ticks: u64) -> (u64, bool, bool) {
    let floor = min_ticks.max(1);
    let ceiling = max_ticks.max(floor);
    if ticks < floor {
        (floor, true, false)
    } else if ticks > ceiling {
        (ceiling, false, true)
    } else {
        (ticks.max(1), false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn ns_tick_conversions_handle_freq_and_fallback() {
        assert_eq!(ns_to_ticks(10_000_000, 1_000_000, 1_000), 10_000);
        assert_eq!(ns_to_ticks(10_000_000, 0, 1_000), 10_000);
        assert_eq!(ticks_to_ns(10_000, 1_000_000), 10_000_000);
        assert_eq!(ticks_to_ns(10_000, 0), 0);
    }

    #[test_case]
    fn clamp_ticks_reports_min_and_max_hits() {
        assert_eq!(clamp_ticks(5, 10, 100), (10, true, false));
        assert_eq!(clamp_ticks(500, 10, 100), (100, false, true));
        assert_eq!(clamp_ticks(50, 10, 100), (50, false, false));
    }
}
