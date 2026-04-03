#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub(crate) fn log_x86_irq_runtime() {
    use aethercore::generated_consts::{CORE_IRQ_STORM_THRESHOLD, CORE_IRQ_STORM_WINDOW_TICKS};
    use aethercore::kernel::syscalls::syscalls_consts::x86;

    aethercore::klog_info!(
        "x86_64 irq policy: storm_threshold={} storm_window_ticks={} timer_vector={} tlb_vector={}",
        CORE_IRQ_STORM_THRESHOLD,
        CORE_IRQ_STORM_WINDOW_TICKS,
        x86::IRQ_TIMER,
        x86::IRQ_TLB_SHOOTDOWN
    );

    let irq = aethercore::hal::idt::irq_dispatch_metrics();
    let timer_dropped = aethercore::kernel::interrupt_guard::dropped_for(x86::IRQ_TIMER);

    aethercore::klog_info!(
        "x86_64 irq stats: total={} timer={} non_timer={} dropped={} dispatch_attempted={} dispatch_handled={} timer_dropped={}",
        irq.total,
        irq.timer,
        irq.non_timer,
        irq.dropped,
        irq.dispatch_attempted,
        irq.dispatch_handled,
        timer_dropped
    );
}
