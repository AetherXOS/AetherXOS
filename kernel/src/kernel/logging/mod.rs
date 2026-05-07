//! AetherXOS Structured Logging System
//! Supports log levels, timestamps, and subsystem filtering.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Panic,
}

pub struct LogMessage {
    pub level: LogLevel,
    pub subsystem: &'static str,
    pub message: &'static str,
    pub timestamp_ns: u64,
}

#[macro_export]
macro_rules! aether_log {
    ($level:expr, $subsystem:expr, $($arg:tt)*) => {
        // In a real system, this would write to a ring buffer or serial
        let ts = 0; // Get timestamp from HAL
        if $level >= $crate::kernel::logging::LogLevel::Info {
            crate::klog_info!("[{}] [{}] {}", ts, $subsystem, format_args!($($arg)*));
        }
    };
}

// Convenience macros
#[macro_export] macro_rules! log_info { ($sub:expr, $($arg:tt)*) => { $crate::aether_log!($crate::kernel::logging::LogLevel::Info, $sub, $($arg)*) }; }
#[macro_export] macro_rules! log_warn { ($sub:expr, $($arg:tt)*) => { $crate::aether_log!($crate::kernel::logging::LogLevel::Warn, $sub, $($arg)*) }; }
#[macro_export] macro_rules! log_err  { ($sub:expr, $($arg:tt)*) => { $crate::aether_log!($crate::kernel::logging::LogLevel::Error, $sub, $($arg)*) }; }
