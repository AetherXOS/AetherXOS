use super::*;
use alloc::string::ToString;

#[test_case]
fn trace_category_string_roundtrip() {
    assert_eq!(TraceCategory::from_str("loader"), Some(TraceCategory::Loader));
    assert_eq!(TraceCategory::Fault.to_string(), "fault");
    assert_eq!(TraceCategory::from_str("bad"), None);
}

#[test_case]
fn trace_ring_records_recent_events_in_order() {
    record("trace.test", "alpha", None, false);
    record("trace.test", "beta", Some(0x44), false);

    let mut recent = [TraceRecord::EMPTY; 4];
    let written = recent_into(&mut recent);
    assert!(written >= 2);
    let last = recent[written - 1];
    let prev = recent[written - 2];
    assert_eq!(prev.scope_str(), "trace.test");
    assert_eq!(prev.stage_str(), "alpha");
    assert_eq!(last.scope_str(), "trace.test");
    assert_eq!(last.stage_str(), "beta");
    assert_eq!(last.value, 0x44);
}

#[test_case]
fn trace_ring_truncates_long_scope_and_stage_names() {
    record(
        "trace.scope.name.is.longer.than.limit",
        "trace.stage.name.is.longer.than.limit",
        None,
        false,
    );
    let mut recent = [TraceRecord::EMPTY; 1];
    let written = recent_into(&mut recent);
    assert_eq!(written, 1);
    assert_eq!(recent[0].scope_len as usize, TRACE_TEXT_LIMIT);
    assert_eq!(recent[0].stage_len as usize, TRACE_TEXT_LIMIT);
}

#[test_case]
fn bytes_preview_records_folded_value_and_length() {
    record_bytes_preview("trace.dump", "preview", &[0x11, 0x22, 0x33, 0x44]);

    let mut recent = [TraceRecord::EMPTY; 4];
    let written = recent_into(&mut recent);
    assert!(written >= 2);
    let preview = recent[written - 2];
    let len = recent[written - 1];
    assert_eq!(preview.scope_str(), "trace.dump");
    assert_eq!(preview.stage_str(), "preview");
    assert_eq!(preview.value, 0x44332211);
    assert_eq!(len.scope_str(), "trace.dump");
    assert_eq!(len.stage_str(), "len");
    assert_eq!(len.value, 4);
}

#[test_case]
fn severity_and_category_are_preserved() {
    record_with_metadata(
        "trace.loader",
        "fault_path",
        Some(0x55),
        false,
        TraceSeverity::Fault,
        TraceCategory::Loader,
    );

    let mut recent = [TraceRecord::EMPTY; 2];
    let written = recent_into(&mut recent);
    assert!(written >= 1);
    let last = recent[written - 1];
    assert_eq!(last.category_str(), "loader");
    assert_ne!(last.flags & TRACE_FLAG_FAULT, 0);
    assert_eq!(last.value, 0x55);
}

#[test_case]
fn category_stats_count_records() {
    record_with_metadata(
        "trace.launch",
        "step",
        None,
        false,
        TraceSeverity::Trace,
        TraceCategory::Launch,
    );
    record_with_metadata(
        "trace.task",
        "step",
        None,
        false,
        TraceSeverity::Trace,
        TraceCategory::Task,
    );
    let stats = category_stats();
    assert!(stats.launch >= 1);
    assert!(stats.task >= 1);
}

#[test_case]
fn kernel_context_records_stage_even_without_runtime_context() {
    record_kernel_context("trace.ctx", "hit", Some(0x55));

    let mut recent = [TraceRecord::EMPTY; 4];
    let written = recent_into(&mut recent);
    assert!(written >= 1);
    let last = recent[written - 1];
    assert_eq!(last.scope_str(), "trace.ctx");
}

#[test_case]
fn trace_stats_report_recent_totals() {
    record("trace.stats", "plain", None, false);
    record("trace.stats", "valued", Some(7), false);
    record("trace.stats", "dump", Some(9), true);
    record_warn("trace.stats", "warn", None);
    record_fault("trace.stats", "fault", None);

    let stats = stats();
    assert!(stats.events >= 5);
    assert!(stats.valued_events >= 2);
    assert!(stats.dump_events >= 1);
    assert!(stats.warn_events >= 1);
    assert!(stats.fault_events >= 1);
    assert!(stats.latest_seq >= 1);
}

#[test_case]
fn recent_records_copy_returns_trace_buffer_snapshot() {
    record("trace.copy", "alpha", None, false);
    let copied = recent_records_copy();
    assert!(copied.iter().any(|record| record.scope_str() == "trace.copy"));
}
