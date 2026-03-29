use super::check;
use super::BootHealthReport;
use crate::generated_consts::{
    AARCH64_IRQ_PER_LINE_LOG_EVERY, AARCH64_IRQ_PER_LINE_STORM_THRESHOLD,
    AARCH64_IRQ_RATE_TRACK_LIMIT, AARCH64_IRQ_STORM_LOG_EVERY, AARCH64_IRQ_STORM_THRESHOLD,
    AARCH64_IRQ_STORM_WINDOW_TICKS, AARCH64_TIMER_JITTER_TOLERANCE_TICKS,
    AARCH64_TIMER_REARM_MAX_TICKS, AARCH64_TIMER_REARM_MIN_TICKS,
};

pub(super) fn run_aarch64_checks(report: &mut BootHealthReport) {
    check(
        report,
        1101,
        AARCH64_IRQ_STORM_WINDOW_TICKS > 0,
        "aarch64 irq_storm_window_ticks must be > 0",
    );
    check(
        report,
        1102,
        AARCH64_IRQ_STORM_THRESHOLD > 0,
        "aarch64 irq_storm_threshold must be > 0",
    );
    check(
        report,
        1103,
        AARCH64_IRQ_STORM_LOG_EVERY > 0,
        "aarch64 irq_storm_log_every must be > 0",
    );
    check(
        report,
        1104,
        AARCH64_TIMER_REARM_MIN_TICKS > 0
            && AARCH64_TIMER_REARM_MIN_TICKS <= AARCH64_TIMER_REARM_MAX_TICKS,
        "aarch64 timer rearm min/max invalid",
    );
    check(
        report,
        1105,
        AARCH64_TIMER_JITTER_TOLERANCE_TICKS > 0,
        "aarch64 timer jitter tolerance must be > 0",
    );
    check(
        report,
        1106,
        AARCH64_IRQ_RATE_TRACK_LIMIT > 0 && AARCH64_IRQ_RATE_TRACK_LIMIT <= 256,
        "aarch64 irq_rate_track_limit out of bounds (1..=256)",
    );
    check(
        report,
        1107,
        AARCH64_IRQ_PER_LINE_STORM_THRESHOLD > 0,
        "aarch64 irq_per_line_storm_threshold must be > 0",
    );
    check(
        report,
        1108,
        AARCH64_IRQ_PER_LINE_LOG_EVERY > 0,
        "aarch64 irq_per_line_log_every must be > 0",
    );
}
