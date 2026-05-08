pub mod log_entry;
pub mod irq_handler;
pub mod retry;
pub mod perf_trace;
pub mod trace_args;
pub mod lock_monitor;
pub mod contracts;

/// Dumps all AOP metrics and statistics gathered by the system.
/// This includes performance traces and lock monitoring data.
pub fn dump_all_aop_stats() {
    crate::klog_info!("================ AOP DIAGNOSTICS DUMP ================");
    perf_trace::dump_metrics();
    lock_monitor::dump_lock_stats();
    crate::klog_info!("======================================================");
}
