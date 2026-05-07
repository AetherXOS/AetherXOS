#[inline(always)]
pub(crate) fn is_sample_boundary(sample: u64, sample_interval: u64) -> bool {
    sample % sample_interval.max(1) == 0
}

#[inline(always)]
pub(crate) fn should_log_now(
    sample: u64,
    sample_interval: u64,
    last_log_sample: u64,
    log_interval_multiplier: u64,
) -> bool {
    let log_period = sample_interval.saturating_mul(log_interval_multiplier);
    sample.saturating_sub(last_log_sample) >= log_period
}

#[cfg(test)]
mod tests {
    use super::{is_sample_boundary, should_log_now};

    #[test]
    fn sample_boundary_handles_zero_interval() {
        assert!(is_sample_boundary(0, 0));
        assert!(is_sample_boundary(5, 0));
    }

    #[test]
    fn sample_boundary_checks_periods() {
        assert!(is_sample_boundary(8, 4));
        assert!(!is_sample_boundary(7, 4));
    }

    #[test]
    fn log_gate_uses_multiplier_and_last_log_sample() {
        assert!(should_log_now(32, 4, 0, 8));
        assert!(!should_log_now(31, 4, 0, 8));
        assert!(!should_log_now(33, 4, 32, 8));
    }
}
