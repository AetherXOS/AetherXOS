pub(super) fn next_round_robin_slice(
    slice_remaining: u64,
    tick_ns: u64,
) -> (u64, bool) {
    if slice_remaining <= tick_ns {
        (0, true)
    } else {
        (slice_remaining - tick_ns, false)
    }
}

#[inline(always)]
pub(super) fn fifo_priority_from_task(task_id: usize) -> u8 {
    (task_id & 0xFF) as u8
}

#[inline(always)]
pub(super) fn fifo_should_preempt(waiting_priority: u8, current_priority: u8) -> bool {
    waiting_priority > current_priority
}

pub(super) fn cfs_should_preempt(
    current_vruntime: u64,
    queued_tasks: &[(Option<usize>, u64)],
    granularity: u64,
) -> bool {
    queued_tasks.iter().any(|(task, vrt)| {
        task.is_some() && current_vruntime.saturating_sub(*vrt) > granularity
    })
}

#[inline(always)]
pub(super) fn lottery_base_tickets_from_raw(raw: u64) -> u32 {
    raw.clamp(1, u32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn round_robin_tick_helper_expires_and_counts_down() {
        assert_eq!(next_round_robin_slice(5, 5), (0, true));
        assert_eq!(next_round_robin_slice(4, 5), (0, true));
        assert_eq!(next_round_robin_slice(9, 5), (4, false));
    }

    #[test_case]
    fn fifo_helpers_preserve_priority_contract() {
        assert_eq!(fifo_priority_from_task(0x12AB), 0xAB);
        assert!(fifo_should_preempt(9, 4));
        assert!(!fifo_should_preempt(4, 9));
        assert!(!fifo_should_preempt(7, 7));
    }

    #[test_case]
    fn cfs_preemption_helper_detects_significant_lag() {
        let queued = [(Some(1), 10), (Some(2), 95), (None, 0)];
        assert!(cfs_should_preempt(100, &queued, 32));
        assert!(!cfs_should_preempt(100, &queued, 128));
    }

    #[test_case]
    fn lottery_ticket_helper_clamps_bounds() {
        assert_eq!(lottery_base_tickets_from_raw(0), 1);
        assert_eq!(lottery_base_tickets_from_raw(9), 9);
        assert_eq!(lottery_base_tickets_from_raw(u64::MAX), u32::MAX);
    }
}
