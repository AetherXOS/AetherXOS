/// Runtime Log Level Filtering and Control
///
/// Provides a configurable logging system that can dynamically adjust
/// which messages are actually emitted based on runtime configuration.
/// This is useful for reducing boot-time spam or enabling detailed
/// tracing only for problematic subsystems.

use core::sync::atomic::{AtomicU8, Ordering};

/// Log level as a numeric value for easy comparison.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevelValue {
    /// Trace: Most verbose; internal flow tracking.
    Trace = 0,
    /// Debug: Detailed information for developers.
    Debug = 1,
    /// Info: General informational messages.
    Info = 2,
    /// Warn: Warning conditions; something unexpected.
    Warn = 3,
    /// Error: Error conditions; serious problems.
    Error = 4,
    /// Panic: Critical failures; kernel panic level.
    Panic = 5,
}

impl LogLevelValue {
    /// Get the numeric value.
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Convert from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevelValue::Trace),
            "debug" => Some(LogLevelValue::Debug),
            "info" => Some(LogLevelValue::Info),
            "warn" => Some(LogLevelValue::Warn),
            "error" => Some(LogLevelValue::Error),
            "panic" => Some(LogLevelValue::Panic),
            _ => None,
        }
    }

    /// Get the name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevelValue::Trace => "TRACE",
            LogLevelValue::Debug => "DEBUG",
            LogLevelValue::Info => "INFO",
            LogLevelValue::Warn => "WARN",
            LogLevelValue::Error => "ERROR",
            LogLevelValue::Panic => "PANIC",
        }
    }
}

/// Global minimum log level. Messages below this level are filtered out.
/// Initially set to Info (suppress Trace and Debug).
static GLOBAL_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevelValue::Info as u8);

/// Per-subsystem log level overrides (e.g., enable detailed debugging for scheduler only).
/// This would typically be backed by a small array or map, but for simplicity
/// we show the concept with a few static examples.
static SCHEDULER_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevelValue::Info as u8);
static MEMORY_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevelValue::Info as u8);
static VFS_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevelValue::Info as u8);

/// Check if a message at `level` should be emitted.
///
/// Returns true if the message passes the global or subsystem-specific filter.
pub fn should_log_at_level(level: &str) -> bool {
    let level_val = match LogLevelValue::from_str(level) {
        Some(lv) => lv,
        None => LogLevelValue::Info, // Default if unrecognized
    };

    let _global_level = LogLevelValue::Info as u8;
    let stored = GLOBAL_LOG_LEVEL.load(Ordering::Relaxed);

    level_val.as_u8() >= stored
}

/// Check if a message for a specific subsystem should be emitted.
pub fn should_log_subsystem(subsystem: &str, level: &str) -> bool {
    let level_val = match LogLevelValue::from_str(level) {
        Some(lv) => lv,
        None => LogLevelValue::Info,
    };

    let subsys_level = match subsystem {
        "scheduler" => SCHEDULER_LOG_LEVEL.load(Ordering::Relaxed),
        "memory" => MEMORY_LOG_LEVEL.load(Ordering::Relaxed),
        "vfs" => VFS_LOG_LEVEL.load(Ordering::Relaxed),
        _ => GLOBAL_LOG_LEVEL.load(Ordering::Relaxed),
    };

    level_val.as_u8() >= subsys_level
}

/// Set the global minimum log level.
///
/// All messages with levels below this are filtered.
pub fn set_global_log_level(level: LogLevelValue) {
    GLOBAL_LOG_LEVEL.store(level.as_u8(), Ordering::Relaxed);
}

/// Set the log level for a specific subsystem (override global level).
pub fn set_subsystem_log_level(subsystem: &str, level: LogLevelValue) {
    match subsystem {
        "scheduler" => SCHEDULER_LOG_LEVEL.store(level.as_u8(), Ordering::Relaxed),
        "memory" => MEMORY_LOG_LEVEL.store(level.as_u8(), Ordering::Relaxed),
        "vfs" => VFS_LOG_LEVEL.store(level.as_u8(), Ordering::Relaxed),
        _ => {
            // Unknown subsystem; silently ignore or log a warning
        }
    }
}

/// Get the current global log level.
pub fn get_global_log_level() -> LogLevelValue {
    let stored = GLOBAL_LOG_LEVEL.load(Ordering::Relaxed);
    match stored {
        0 => LogLevelValue::Trace,
        1 => LogLevelValue::Debug,
        2 => LogLevelValue::Info,
        3 => LogLevelValue::Warn,
        4 => LogLevelValue::Error,
        _ => LogLevelValue::Panic,
    }
}

/// Get the log level for a specific subsystem.
pub fn get_subsystem_log_level(subsystem: &str) -> LogLevelValue {
    let stored = match subsystem {
        "scheduler" => SCHEDULER_LOG_LEVEL.load(Ordering::Relaxed),
        "memory" => MEMORY_LOG_LEVEL.load(Ordering::Relaxed),
        "vfs" => VFS_LOG_LEVEL.load(Ordering::Relaxed),
        _ => GLOBAL_LOG_LEVEL.load(Ordering::Relaxed),
    };

    match stored {
        0 => LogLevelValue::Trace,
        1 => LogLevelValue::Debug,
        2 => LogLevelValue::Info,
        3 => LogLevelValue::Warn,
        4 => LogLevelValue::Error,
        _ => LogLevelValue::Panic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevelValue::Error > LogLevelValue::Info);
        assert!(LogLevelValue::Trace < LogLevelValue::Debug);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevelValue::from_str("trace"), Some(LogLevelValue::Trace));
        assert_eq!(LogLevelValue::from_str("ERROR"), Some(LogLevelValue::Error));
        assert_eq!(LogLevelValue::from_str("invalid"), None);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevelValue::Info.as_str(), "INFO");
        assert_eq!(LogLevelValue::Warn.as_str(), "WARN");
    }

    #[test]
    fn test_should_log_filtering() {
        // Set global level to Warn
        set_global_log_level(LogLevelValue::Warn);

        // Info messages should be filtered
        assert!(!should_log_at_level("info"));
        
        // Warn and Error should pass
        assert!(should_log_at_level("warn"));
        assert!(should_log_at_level("error"));

        // Reset to default
        set_global_log_level(LogLevelValue::Info);
    }

    #[test]
    fn test_subsystem_log_level_override() {
        // Globally set to Warn
        set_global_log_level(LogLevelValue::Warn);

        // Override scheduler to Trace
        set_subsystem_log_level("scheduler", LogLevelValue::Trace);

        // Trace for scheduler should pass
        assert!(should_log_subsystem("scheduler", "trace"));

        // Trace for vfs should fail (uses global Warn)
        assert!(!should_log_subsystem("vfs", "trace"));

        // Reset
        set_global_log_level(LogLevelValue::Info);
        set_subsystem_log_level("scheduler", LogLevelValue::Info);
    }

    #[test]
    fn test_get_log_level() {
        set_global_log_level(LogLevelValue::Debug);
        assert_eq!(get_global_log_level(), LogLevelValue::Debug);

        set_subsystem_log_level("memory", LogLevelValue::Trace);
        assert_eq!(get_subsystem_log_level("memory"), LogLevelValue::Trace);

        // Reset
        set_global_log_level(LogLevelValue::Info);
        set_subsystem_log_level("memory", LogLevelValue::Info);
    }
}

// Integration: How this would be used with the core logging facade:
//
// Before (No filtering):
//   crate::core::log::debug("some debug message"); // Always emitted
//
// After (With runtime filtering):
//   if crate::core::log_filter::should_log_at_level("debug") {
//       crate::core::log::debug("some debug message");
//   }
//
// Or with macro support (future):
//   #[log_entry(debug, filtered)]  // Macro checks filtering
//   fn my_function() { }
