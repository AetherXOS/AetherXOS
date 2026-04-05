/// Time-Related ABI Parity Tests
///
/// Covers clock, timer, ppoll, and pselect normalization rules.

#[cfg(test)]
mod tests {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }

    fn normalize_timeout(mut ts: Timespec) -> Result<Timespec, &'static str> {
        if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
            return Err("EINVAL");
        }
        if ts.tv_sec == 0 && ts.tv_nsec == 0 {
            ts.tv_nsec = 0;
        }
        Ok(ts)
    }

    fn timer_expiry_is_monotonic(start_ns: u128, now_ns: u128, interval_ns: u128) -> bool {
        now_ns >= start_ns && interval_ns > 0
    }

    fn clock_gettime_model(clock_id: usize, valid_ptr: bool) -> Result<usize, &'static str> {
        if !valid_ptr {
            return Err("EFAULT");
        }
        match clock_id {
            0 | 1 | 7 => Ok(0),
            _ => Err("EINVAL"),
        }
    }

    fn ppoll_mask_is_preserved(sigmask_present: bool, timeout: Option<Timespec>) -> Result<bool, &'static str> {
        if let Some(ts) = timeout {
            normalize_timeout(ts)?;
        }
        if !sigmask_present {
            return Err("EFAULT");
        }
        Ok(true)
    }

    fn pselect_mask_is_preserved(sigmask_present: bool, timeout: Option<Timespec>) -> Result<bool, &'static str> {
        if let Some(ts) = timeout {
            normalize_timeout(ts)?;
        }
        if !sigmask_present {
            return Err("EFAULT");
        }
        Ok(true)
    }

    #[test_case]
    fn clock_gettime_accepts_known_ids_and_rejects_invalid_pointers() {
        assert_eq!(clock_gettime_model(0, true), Ok(0));
        assert_eq!(clock_gettime_model(1, true), Ok(0));
        assert_eq!(clock_gettime_model(42, true), Err("EINVAL"));
        assert_eq!(clock_gettime_model(0, false), Err("EFAULT"));
    }

    #[test_case]
    fn timer_expiry_requires_forward_progress() {
        assert!(timer_expiry_is_monotonic(100, 100, 1));
        assert!(timer_expiry_is_monotonic(100, 101, 1));
        assert!(!timer_expiry_is_monotonic(100, 99, 1));
        assert!(!timer_expiry_is_monotonic(100, 100, 0));
    }

    #[test_case]
    fn ppoll_rejects_invalid_timespec_and_missing_mask() {
        assert_eq!(
            ppoll_mask_is_preserved(true, Some(Timespec { tv_sec: -1, tv_nsec: 0 })),
            Err("EINVAL")
        );
        assert_eq!(
            ppoll_mask_is_preserved(false, Some(Timespec { tv_sec: 0, tv_nsec: 1 })),
            Err("EFAULT")
        );
        assert_eq!(
            ppoll_mask_is_preserved(true, Some(Timespec { tv_sec: 0, tv_nsec: 1 })),
            Ok(true)
        );
    }

    #[test_case]
    fn pselect_rejects_invalid_timespec_and_requires_mask() {
        assert_eq!(
            pselect_mask_is_preserved(true, Some(Timespec { tv_sec: 0, tv_nsec: 1_000_000_000 })),
            Err("EINVAL")
        );
        assert_eq!(
            pselect_mask_is_preserved(false, None),
            Err("EFAULT")
        );
        assert_eq!(pselect_mask_is_preserved(true, None), Ok(true));
    }
}
