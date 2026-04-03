#[cfg(target_arch = "aarch64")]
pub(crate) fn log_aarch64_exception_runtime() {
    use aethercore::generated_consts::{
        AARCH64_EXCEPTION_KILL_USER_ASYNC, AARCH64_EXCEPTION_KILL_USER_SYNC,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC, AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
        AARCH64_GIC_CPU_PRIORITY_MASK, AARCH64_IRQ_PER_LINE_LOG_EVERY,
        AARCH64_IRQ_PER_LINE_STORM_THRESHOLD, AARCH64_IRQ_RATE_TRACK_LIMIT,
        AARCH64_IRQ_STORM_LOG_EVERY, AARCH64_IRQ_STORM_THRESHOLD, AARCH64_IRQ_STORM_WINDOW_TICKS,
        AARCH64_TIMER_JITTER_TOLERANCE_TICKS, AARCH64_TIMER_REARM_MAX_TICKS,
        AARCH64_TIMER_REARM_MIN_TICKS,
    };

    aethercore::klog_info!(
        "AArch64 exception policy: kill_user_sync={} kill_user_async={} panic_kernel_sync={} panic_kernel_async={} gic_pmr={} irq_track_limit={} irq_storm_window={} irq_storm_threshold={} irq_storm_log_every={} irq_line_storm_threshold={} irq_line_log_every={} timer_rearm_min={} timer_rearm_max={} timer_jitter_tol={}",
        AARCH64_EXCEPTION_KILL_USER_SYNC,
        AARCH64_EXCEPTION_KILL_USER_ASYNC,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC,
        AARCH64_GIC_CPU_PRIORITY_MASK.min(0xFF),
        AARCH64_IRQ_RATE_TRACK_LIMIT,
        AARCH64_IRQ_STORM_WINDOW_TICKS,
        AARCH64_IRQ_STORM_THRESHOLD,
        AARCH64_IRQ_STORM_LOG_EVERY,
        AARCH64_IRQ_PER_LINE_STORM_THRESHOLD,
        AARCH64_IRQ_PER_LINE_LOG_EVERY,
        AARCH64_TIMER_REARM_MIN_TICKS,
        AARCH64_TIMER_REARM_MAX_TICKS,
        AARCH64_TIMER_JITTER_TOLERANCE_TICKS
    );

    let ex = aethercore::hal::exception::stats();
    aethercore::klog_info!(
        "AArch64 exception stats: sync={} fiq={} serror={} user_abort={} kernel_abort={} user_fatal_sync={} user_fatal_async={} kernel_fatal_async={} irq_total={} irq_spurious={} irq_storm_windows={} irq_suppressed={} timer_irq={} timer_jitter={} irq_track_limit={} irq_hot={} irq_hot_total={} irq_hot_storms={} irq_hot_suppressed={} gic_pmr={}",
        ex.sync_exceptions,
        ex.fiq_exceptions,
        ex.serror_exceptions,
        ex.user_abort_exceptions,
        ex.kernel_abort_exceptions,
        ex.user_fatal_sync_exceptions,
        ex.user_fatal_async_exceptions,
        ex.kernel_fatal_async_exceptions,
        ex.irq_total_exceptions,
        ex.irq_spurious_exceptions,
        ex.irq_storm_windows,
        ex.irq_storm_suppressed_logs,
        ex.timer_irq_count,
        ex.timer_irq_jitter_events,
        ex.irq_track_limit,
        ex.hottest_irq_line,
        ex.hottest_irq_total,
        ex.hottest_irq_storm_events,
        ex.hottest_irq_suppressed_logs,
        ex.gic_cpu_priority_mask
    );

    let timer = aethercore::hal::timer::GenericTimer::stats();
    aethercore::klog_info!(
        "AArch64 timer stats: freq={} last_ticks={} clamp_min={} clamp_max={}",
        timer.frequency_hz,
        timer.last_programmed_ticks,
        timer.clamp_min_hits,
        timer.clamp_max_hits
    );
}
