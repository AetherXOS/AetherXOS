use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub fn wait_for_pending_acks(pending: &AtomicUsize, timeout_spins: usize) -> Option<usize> {
    let timeout = timeout_spins.max(1);
    let mut spins = 0usize;
    while pending.load(Ordering::Acquire) > 0 {
        core::hint::spin_loop();
        spins = spins.saturating_add(1);
        if spins >= timeout {
            return Some(pending.load(Ordering::Relaxed));
        }
    }
    None
}

pub fn acknowledge_pending(pending: &AtomicUsize) {
    let _ = pending.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
        value.checked_sub(1)
    });
}

pub fn wait_for_ready_count(ready: &AtomicU32, expected: u32, timeout_spins: usize) -> bool {
    let timeout = timeout_spins.max(1);
    for _ in 0..timeout {
        if ready.load(Ordering::Acquire) >= expected {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(all(test, target_os = "none"), test_case)]
    #[cfg_attr(not(all(test, target_os = "none")), test)]
    fn ack_wait_and_ready_helpers_report_completion_and_timeout() {
        let pending = AtomicUsize::new(2);
        assert_eq!(wait_for_pending_acks(&pending, 2), Some(2));
        acknowledge_pending(&pending);
        acknowledge_pending(&pending);
        assert_eq!(wait_for_pending_acks(&pending, 2), None);

        let ready = AtomicU32::new(1);
        assert!(!wait_for_ready_count(&ready, 2, 2));
        ready.store(2, Ordering::Release);
        assert!(wait_for_ready_count(&ready, 2, 2));
    }
}
