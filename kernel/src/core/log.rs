/// Core logging facade for AOP and system diagnostics.
/// This module provides a common interface for cross-cutting logging concerns.
/// 
/// # Thread Safety
/// On single-core systems or with proper locking, this can be safely called from
/// IRQ handlers and other interrupt contexts.

use crate::hal;
use crate::core::log_filter;
use alloc::format;

/// Log an event with the specified level.
/// 
/// Levels: "trace", "debug", "info", "warn", "error"
/// 
/// This function respects the runtime log level filter configured via
/// `log_filter::set_global_log_level()` and per-subsystem settings.
/// 
/// # Safety
/// This function internally calls `hal::serial::write_raw()` which must be
/// initialized before use. Calling from IRQ handlers is permitted if the
/// serial driver is interrupt-safe.
pub fn log_event(level: &str, message: &str) {
    // Check if this log level should be output
    if !log_filter::should_log_at_level(level) {
        return;
    }
    
    // Format: [TIMESTAMP] [LEVEL] message
    // For now, we skip timestamp to avoid cycle_count() dependency loops.
    let formatted = format!("[{}] {}\n", level, message);
    
    // SAFETY: hal::serial::write_raw is safe to call from IRQ contexts
    // if the serial device is already initialized and doesn't use blocking locks.
    hal::serial::write_raw(&formatted);
}

/// Log with trace level.
pub fn trace(message: &str) {
    log_event("trace", message);
}

/// Log with debug level.
pub fn debug(message: &str) {
    log_event("debug", message);
}

/// Log with info level.
pub fn info(message: &str) {
    log_event("info", message);
}

/// Log with warn level.
pub fn warn(message: &str) {
    log_event("warn", message);
}

/// Log with error level.
pub fn error(message: &str) {
    log_event("error", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_levels() {
        // Verify that log functions can be called without panicking.
        // Actual output testing requires serial device initialization.
        trace("test trace");
        debug("test debug");
        info("test info");
        warn("test warn");
        error("test error");
    }
}
