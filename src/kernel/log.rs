use crate::kernel::sync::IrqSafeMutex;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write;
use lazy_static::lazy_static;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

#[inline(always)]
pub fn enabled(level: Level) -> bool {
    let level_num = level as u8;

    if !crate::config::KernelConfig::is_telemetry_enabled() && level_num >= Level::Info as u8 {
        return false;
    }

    if crate::generated_consts::BOOT_QUIET && level_num >= Level::Info as u8 {
        return false;
    }

    level_num <= crate::config::KernelConfig::log_level_num()
}

const LOG_BUFFER_LIMIT: usize = 16384;

lazy_static! {
    static ref LOG_BUFFER: IrqSafeMutex<Vec<u8>> =
        IrqSafeMutex::new(Vec::with_capacity(LOG_BUFFER_LIMIT));
}

#[inline(always)]
pub fn log(level: Level, args: fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }

    let tag = match level {
        Level::Error => "ERROR",
        Level::Warn => "WARN",
        Level::Info => "INFO",
        Level::Debug => "DEBUG",
        Level::Trace => "TRACE",
    };

    let s = alloc::format!("[{}] {}\n", tag, args);
    let bytes = s.as_bytes();

    {
        let mut buffer = LOG_BUFFER.lock();
        if buffer.len() + bytes.len() > LOG_BUFFER_LIMIT {
            let to_remove = (buffer.len() + bytes.len()) - LOG_BUFFER_LIMIT;
            if to_remove < buffer.len() {
                buffer.drain(0..to_remove);
            } else {
                buffer.clear();
            }
        }
        buffer.extend_from_slice(bytes);
    }

    let mut serial = crate::hal::serial::SERIAL1.lock();
    let _ = serial.write_str(&s);
}

pub fn read_to_buffer(out: &mut [u8]) -> usize {
    let buffer = LOG_BUFFER.lock();
    let len = buffer.len().min(out.len());
    out[..len].copy_from_slice(&buffer[buffer.len() - len..]);
    len
}

pub fn get_total_size() -> usize {
    LOG_BUFFER.lock().len()
}

#[macro_export]
macro_rules! klog_error {
    ($($arg:tt)*) => {
        $crate::kernel::log::log($crate::kernel::log::Level::Error, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog_warn {
    ($($arg:tt)*) => {
        $crate::kernel::log::log($crate::kernel::log::Level::Warn, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog_info {
    ($($arg:tt)*) => {
        $crate::kernel::log::log($crate::kernel::log::Level::Info, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog_debug {
    ($($arg:tt)*) => {
        $crate::kernel::log::log($crate::kernel::log::Level::Debug, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog_trace {
    ($($arg:tt)*) => {
        $crate::kernel::log::log($crate::kernel::log::Level::Trace, format_args!($($arg)*))
    };
}
