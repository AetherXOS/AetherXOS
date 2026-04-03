#[inline(always)]
pub fn trace_dispatched(scope: &str, irq: u64, kind: &str, enabled: bool) {
    if enabled {
        crate::klog_trace!("{} IRQ {} kind={} dispatched", scope, irq, kind);
    }
}

#[inline(always)]
pub fn trace_dropped_by_storm(scope: &str, irq: u64, kind: &str, enabled: bool) {
    if enabled {
        crate::klog_trace!(
            "{} IRQ {} kind={} dropped by storm protection",
            scope,
            irq,
            kind
        );
    }
}

#[inline(always)]
pub fn debug_storm_window(scope: &str, irq: u64, kind: &str, window_events: u64, in_storm: bool) {
    crate::klog_debug!(
        "{} IRQ {} kind={} window_events={} storm={}",
        scope,
        irq,
        kind,
        window_events,
        in_storm
    );
}

#[inline(always)]
pub fn warn_line_storm(
    scope: &str,
    irq: u64,
    kind: &str,
    line_window_events: u64,
    threshold: u64,
) {
    crate::klog_warn!(
        "{} IRQ line storm irq={} kind={} line_window_events={} threshold={}",
        scope,
        irq,
        kind,
        line_window_events,
        threshold
    );
}
