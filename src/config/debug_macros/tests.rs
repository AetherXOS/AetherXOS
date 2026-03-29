use super::*;
use alloc::string::ToString;

#[test_case]
fn category_str_representation() {
    assert_eq!(ObservabilityCategory::Core.as_str(), "CORE");
    assert_eq!(ObservabilityCategory::Boot.as_str(), "BOOT");
    assert_eq!(ObservabilityCategory::Memory.as_str(), "MEMORY");
    assert_eq!(ObservabilityCategory::Scheduler.as_str(), "SCHED");
}

#[test_case]
fn category_from_str_roundtrip() {
    assert_eq!(ObservabilityCategory::from_str("BOOT"), Some(ObservabilityCategory::Boot));
    assert_eq!(ObservabilityCategory::from_str("NET"), Some(ObservabilityCategory::Network));
    assert_eq!(ObservabilityCategory::from_str("INVALID"), None);
    assert_eq!(ObservabilityCategory::Task.to_string(), "TASK");
}

#[test_case]
fn category_u8_conversion() {
    assert_eq!(ObservabilityCategory::Core.as_u8(), 0);
    assert_eq!(ObservabilityCategory::Boot.as_u8(), 1);
    assert_eq!(ObservabilityCategory::Memory.as_u8(), 4);
}

#[test_case]
fn category_from_u8() {
    assert_eq!(
        ObservabilityCategory::from_u8(0),
        Some(ObservabilityCategory::Core)
    );
    assert_eq!(
        ObservabilityCategory::from_u8(4),
        Some(ObservabilityCategory::Memory)
    );
    assert_eq!(ObservabilityCategory::from_u8(99), None);
}

#[test_case]
fn autonomous_serial_format() {
    let msg = serial_autonomous(ObservabilityCategory::Boot, "ready");
    assert_eq!(msg, "[BOOT] ready\n");
}

#[test_case]
fn autonomous_hex_format() {
    let msg = serial_autonomous_hex(ObservabilityCategory::Memory, "frame", 0x1000);
    assert_eq!(msg, "[MEMORY] frame=0x1000\n");
}

#[test_case]
fn autonomous_trace_format() {
    let msg = trace_autonomous(ObservabilityCategory::Task, "fork");
    assert_eq!(msg, "[TASK] fork\n");
}
