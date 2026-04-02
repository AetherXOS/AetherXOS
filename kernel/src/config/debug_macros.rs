//! Autonomous Debug/Serial Emission System
//! 
//! Provides compile-time and runtime controlled observability with:
//! - Automatic category-based prefix generation
//! - Built-in newline handling (no more manual \n)
//! - Granular section/region/category-based gating
//! - Compile-time optimization (dead code elimination when disabled)
//!
//! # Categories
//! - Core: Core kernel initialization and bootstrap
//! - Boot: Boot sequence and bootloader handoff
//! - Loader: Module/executable loader
//! - Task: Task/process management
//! - Memory: Memory management (allocators, paging)
//! - Scheduler: Task scheduling and load balancing
//! - Fault: Fault handling (exceptions, panics)
//! - Driver: Driver operations
//! - Io: I/O subsystem
//! - Network: Network subsystem
//!
//! # Usage Examples
//!
//! ```ignore
//! // Autonomous serial emit - prefix and newline generated automatically
//! let msg = serial_autonomous(Boot, "x86_64 ap cpu id ready");
//! // Output: "[BOOT] x86_64 ap cpu id ready\n"
//!
//! // With formatting
//! let msg = serial_autonomous_fmt(Memory, &format_args!("allocated {} bytes", size));
//!
//! // Hexadecimal values
//! let msg = serial_autonomous_hex(Memory, "frame_addr", 0x1000);
//! // Output: "[MEMORY] frame_addr=0x1000\n"
//! ```

use alloc::string::String;

/// Category-based observability selector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservabilityCategory {
    /// Core kernel initialization and bootstrap
    Core = 0,
    /// Boot sequence and bootloader handoff
    Boot = 1,
    /// Module/executable loader
    Loader = 2,
    /// Task/process management
    Task = 3,
    /// Memory management (allocators, paging)
    Memory = 4,
    /// Task scheduling and load balancing
    Scheduler = 5,
    /// Fault handling (exceptions, panics)
    Fault = 6,
    /// Driver operations
    Driver = 7,
    /// I/O subsystem
    Io = 8,
    /// Network subsystem
    Network = 9,
}

impl_enum_u8_option_conversions!(ObservabilityCategory {
    Core,
    Boot,
    Loader,
    Task,
    Memory,
    Scheduler,
    Fault,
    Driver,
    Io,
    Network,
});

impl_enum_str_conversions!(ObservabilityCategory {
    Core => "CORE",
    Boot => "BOOT",
    Loader => "LOADER",
    Task => "TASK",
    Memory => "MEMORY",
    Scheduler => "SCHED",
    Fault => "FAULT",
    Driver => "DRIVER",
    Io => "IO",
    Network => "NET",
});

impl ObservabilityCategory {
    /// Get numeric value
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self.to_u8()
    }

    #[inline(always)]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Core),
            1 => Some(Self::Boot),
            2 => Some(Self::Loader),
            3 => Some(Self::Task),
            4 => Some(Self::Memory),
            5 => Some(Self::Scheduler),
            6 => Some(Self::Fault),
            7 => Some(Self::Driver),
            8 => Some(Self::Io),
            9 => Some(Self::Network),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "CORE" => Some(Self::Core),
            "BOOT" => Some(Self::Boot),
            "LOADER" => Some(Self::Loader),
            "TASK" => Some(Self::Task),
            "MEMORY" => Some(Self::Memory),
            "SCHED" => Some(Self::Scheduler),
            "FAULT" => Some(Self::Fault),
            "DRIVER" => Some(Self::Driver),
            "IO" => Some(Self::Io),
            "NET" => Some(Self::Network),
            _ => None,
        }
    }
}

impl core::fmt::Display for ObservabilityCategory {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for ObservabilityCategory {
    type Err = ::aethercore_common::result::ParseEnumError;

    #[inline(always)]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        self::ObservabilityCategory::from_str(value).ok_or_else(|| {
            ::aethercore_common::result::ParseEnumError::new(
                "ObservabilityCategory",
                value,
                &["CORE", "BOOT", "LOADER", "TASK", "MEMORY", "SCHED", "FAULT", "DRIVER", "IO", "NET"],
            )
        })
    }
}

/// Check if a category is enabled at compile-time (via Cargo.toml features)
/// 
/// Returns true if either debug_observability_{category_name} or debug_observability_all feature is enabled
#[inline(always)]
pub const fn is_category_enabled_compile_time(category: ObservabilityCategory) -> bool {
    match category {
        ObservabilityCategory::Core => cfg!(any(
            feature = "debug_observability_core",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Boot => cfg!(any(
            feature = "debug_observability_boot",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Loader => cfg!(any(
            feature = "debug_observability_loader",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Task => cfg!(any(
            feature = "debug_observability_task",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Memory => cfg!(any(
            feature = "debug_observability_memory",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Scheduler => cfg!(any(
            feature = "debug_observability_scheduler",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Fault => cfg!(any(
            feature = "debug_observability_fault",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Driver => cfg!(any(
            feature = "debug_observability_driver",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Io => cfg!(any(
            feature = "debug_observability_io",
            feature = "debug_observability_all"
        )),
        ObservabilityCategory::Network => cfg!(any(
            feature = "debug_observability_network",
            feature = "debug_observability_all"
        )),
    }
}

/// Format autonomous serial message with category prefix and automatic newline
/// 
/// Generates: [CATEGORY] message\n
/// 
/// This replaces manual messages like "[EARLY SERIAL] x86_64 ap cpu id ready\n"
/// with autonomous generation that handles the prefix and newline automatically.
#[allow(dead_code)]
#[inline]
pub fn serial_autonomous(category: ObservabilityCategory, message: &str) -> String {
    alloc::format!("[{}] {}\n", category.as_str(), message)
}

/// Format autonomous message with hexadecimal value
///
/// Generates: [CATEGORY] key=0xvalue\n
#[allow(dead_code)]
#[inline]
pub fn serial_autonomous_hex(category: ObservabilityCategory, key: &str, value: u64) -> String {
    alloc::format!("[{}] {}=0x{:x}\n", category.as_str(), key, value)
}

/// Format autonomous message with formatted arguments
///
/// Generates: [CATEGORY] formatted_message\n
#[allow(dead_code)]
#[inline]
pub fn serial_autonomous_fmt(category: ObservabilityCategory, args: core::fmt::Arguments) -> String {
    alloc::format!("[{}] {}\n", category.as_str(), args)
}

/// Format autonomous debug trace message with category prefix and automatic newline
///
/// Generates: [CATEGORY] message\n
#[allow(dead_code)]
#[inline]
pub fn trace_autonomous(category: ObservabilityCategory, message: &str) -> String {
    alloc::format!("[{}] {}\n", category.as_str(), message)
}

/// Format autonomous trace with hexadecimal value
///
/// Generates: [CATEGORY] key=0xvalue\n
#[allow(dead_code)]
#[inline]
pub fn trace_autonomous_hex(category: ObservabilityCategory, key: &str, value: u64) -> String {
    alloc::format!("[{}] {}=0x{:x}\n", category.as_str(), key, value)
}

#[cfg(all(test, target_os = "none"))]
#[path = "debug_macros/tests.rs"]
mod tests;

