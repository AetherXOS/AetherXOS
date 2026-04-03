use super::*;

use crate::generated_consts::{
    AARCH64_IRQ_PER_LINE_LOG_EVERY, AARCH64_IRQ_PER_LINE_STORM_THRESHOLD,
    AARCH64_IRQ_RATE_TRACK_LIMIT, AARCH64_IRQ_STORM_LOG_EVERY, AARCH64_IRQ_STORM_THRESHOLD,
    AARCH64_IRQ_STORM_WINDOW_TICKS,
};
use crate::hal::common::irq::{hottest_counter_index, reset_window, storm_decision, tracked_limit};

pub(super) const MAX_TRACKED_IRQS: usize = 256;

#[derive(Debug, Clone)]
pub(in super::super) struct IrqRateTracker {
    total: [u64; MAX_TRACKED_IRQS],
    window_start: [u64; MAX_TRACKED_IRQS],
    window_count: [u64; MAX_TRACKED_IRQS],
    storm_events: [u64; MAX_TRACKED_IRQS],
    suppressed_logs: [u64; MAX_TRACKED_IRQS],
}

impl IrqRateTracker {
    pub(in super::super) const fn new() -> Self {
        Self {
            total: [0; MAX_TRACKED_IRQS],
            window_start: [0; MAX_TRACKED_IRQS],
            window_count: [0; MAX_TRACKED_IRQS],
            storm_events: [0; MAX_TRACKED_IRQS],
            suppressed_logs: [0; MAX_TRACKED_IRQS],
        }
    }
}

pub(super) struct IrqStormState {
    now_counter: u64,
    window_ticks: u64,
    global_threshold: u64,
    global_log_every: u64,
    tracked_limit: usize,
    per_line_threshold: u64,
    per_line_log_every: u64,
}

impl IrqStormState {
    pub(super) fn new(now_counter: u64) -> Self {
        Self {
            now_counter,
            window_ticks: AARCH64_IRQ_STORM_WINDOW_TICKS.max(1),
            global_threshold: AARCH64_IRQ_STORM_THRESHOLD.max(1),
            global_log_every: AARCH64_IRQ_STORM_LOG_EVERY.max(1),
            tracked_limit: tracked_limit(AARCH64_IRQ_RATE_TRACK_LIMIT, MAX_TRACKED_IRQS),
            per_line_threshold: AARCH64_IRQ_PER_LINE_STORM_THRESHOLD.max(1),
            per_line_log_every: AARCH64_IRQ_PER_LINE_LOG_EVERY.max(1),
        }
    }

    pub(super) fn per_line_threshold(&self) -> u64 {
        self.per_line_threshold
    }

    pub(super) fn record_global(&self) -> (u64, crate::hal::common::irq::StormDecision) {
        let start = IRQ_WINDOW_START_COUNTER.load(Ordering::Relaxed);
        if reset_window(start, self.now_counter, self.window_ticks) {
            IRQ_WINDOW_START_COUNTER.store(self.now_counter, Ordering::Relaxed);
            IRQ_WINDOW_EVENT_COUNT.store(0, Ordering::Relaxed);
        }

        let in_window = IRQ_WINDOW_EVENT_COUNT
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let decision = storm_decision(
            in_window,
            self.global_threshold,
            self.global_log_every,
            true,
        );
        if decision.first_storm_event {
            IRQ_STORM_WINDOWS.fetch_add(1, Ordering::Relaxed);
        } else if decision.suppressed_log {
            IRQ_STORM_SUPPRESSED_LOGS.fetch_add(1, Ordering::Relaxed);
        }

        (in_window, decision)
    }

    pub(super) fn record_per_line(
        &self,
        irq_id: u32,
    ) -> Option<(u64, crate::hal::common::irq::StormDecision)> {
        if (irq_id as usize) >= self.tracked_limit {
            return None;
        }

        let idx = irq_id as usize;
        let mut tracker = IRQ_RATE_TRACKER.lock();
        tracker.total[idx] = tracker.total[idx].saturating_add(1);

        let start = tracker.window_start[idx];
        if reset_window(start, self.now_counter, self.window_ticks) {
            tracker.window_start[idx] = self.now_counter;
            tracker.window_count[idx] = 0;
        }

        tracker.window_count[idx] = tracker.window_count[idx].saturating_add(1);
        let line_count = tracker.window_count[idx];
        let decision = storm_decision(
            line_count,
            self.per_line_threshold,
            self.per_line_log_every,
            false,
        );
        if decision.first_storm_event {
            tracker.storm_events[idx] = tracker.storm_events[idx].saturating_add(1);
        } else if decision.suppressed_log {
            tracker.suppressed_logs[idx] = tracker.suppressed_logs[idx].saturating_add(1);
        }

        Some((line_count, decision))
    }
}

pub(in super::super) fn hottest_irq_snapshot() -> (usize, u64, u64, u64, usize) {
    let tracked = tracked_limit(AARCH64_IRQ_RATE_TRACK_LIMIT, MAX_TRACKED_IRQS);
    let tracker = IRQ_RATE_TRACKER.lock();
    let best_idx = hottest_counter_index(&tracker.total[..tracked]);
    (
        best_idx,
        tracker.total[best_idx],
        tracker.storm_events[best_idx],
        tracker.suppressed_logs[best_idx],
        tracked,
    )
}
