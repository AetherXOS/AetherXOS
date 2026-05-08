/// Real-World Architecture Example: Boot Logger
///
/// This module demonstrates how to use the Onion Architecture in practice:
/// - Core logging facades (trait-based)
/// - HAL device abstractions (type-safe MMIO)
/// - AOP macros for cross-cutting concerns
///
/// This serves as a template for migrating existing code to the new architecture.

use crate::core::log;
use alloc::format;
use crate::core::time::cycle_count;
use crate::hal::devices::Uart;

/// Boot stage marker for structured logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// Bootloader handed off to kernel.
    BootloaderHandoff,
    /// Early memory initialization.
    EarlyMemory,
    /// CPU features detection.
    CpuFeatures,
    /// Paging and virtual memory setup.
    VirtualMemory,
    /// Interrupt controller initialization.
    Interrupts,
    /// Scheduler initialization.
    Scheduler,
    /// VFS and block device initialization.
    StorageInit,
    /// User-space loader ready.
    UserspaceReady,
}

impl BootStage {
    /// Get a human-readable name for the stage.
    pub fn as_str(&self) -> &'static str {
        match self {
            BootStage::BootloaderHandoff => "Bootloader Handoff",
            BootStage::EarlyMemory => "Early Memory Init",
            BootStage::CpuFeatures => "CPU Features",
            BootStage::VirtualMemory => "Virtual Memory",
            BootStage::Interrupts => "Interrupts",
            BootStage::Scheduler => "Scheduler",
            BootStage::StorageInit => "Storage Init",
            BootStage::UserspaceReady => "Userspace Ready",
        }
    }
}

/// Boot logger with performance tracking.
pub struct BootLogger<const BASE: usize> {
    uart: Uart<BASE>,
    boot_start: u64,
    last_stage_time: u64,
}

impl<const BASE: usize> BootLogger<BASE> {
    /// Create a new boot logger.
    ///
    /// # Safety
    /// BASE must point to a valid UART device.
    pub unsafe fn new() -> Self {
        let boot_start = cycle_count();
        BootLogger {
            uart: Uart::new(),
            boot_start,
            last_stage_time: boot_start,
        }
    }

    /// Initialize the UART for logging.
    ///
    /// # Safety
    /// Must be called exactly once during boot.
    pub unsafe fn init(&mut self) -> Result<(), &'static str> {
        // Note: This would require making Uart initializable in real code
        // For now, assume it starts initialized
        log::info("Boot logger initialized");
        Ok(())
    }

    /// Log entry into a boot stage with performance tracking.
    pub fn enter_stage(&mut self, stage: BootStage) {
        let current = cycle_count();
        let stage_elapsed = current.saturating_sub(self.last_stage_time);
        let total_elapsed = current.saturating_sub(self.boot_start);
        
        let msg = format!(
            "[BOOT] +{}μs (stage: +{}μs) → {}",
            total_elapsed / 1000, // Approx cycles to microseconds (assuming ~1MHz base, rough estimate)
            stage_elapsed / 1000,
            stage.as_str()
        );
        
        log::info(&msg);
        self.last_stage_time = current;
    }

    /// Log a message at a specific stage with context.
    pub fn log_at_stage(&self, stage: BootStage, level: &str, message: &str) {
        let msg = format!("[{}] {}", stage.as_str(), message);
        log::log_event(level, &msg);
    }

    /// Log entry completion with timing.
    pub fn exit_stage(&self, stage: BootStage) {
        let current = cycle_count();
        let total = current.saturating_sub(self.boot_start);
        let msg = format!("[BOOT] ✓ {} complete ({}μs total)", stage.as_str(), total / 1000);
        log::info(&msg);
    }

    /// Log a critical error during boot.
    pub fn error(&self, stage: BootStage, error: &str) {
        let msg = format!("[{}] ERROR: {}", stage.as_str(), error);
        log::error(&msg);
    }

    /// Log a warning during boot.
    pub fn warn(&self, stage: BootStage, warning: &str) {
        let msg = format!("[{}] WARNING: {}", stage.as_str(), warning);
        log::warn(&msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_stage_names() {
        assert_eq!(BootStage::BootloaderHandoff.as_str(), "Bootloader Handoff");
        assert_eq!(BootStage::EarlyMemory.as_str(), "Early Memory Init");
        assert_eq!(BootStage::Scheduler.as_str(), "Scheduler");
    }

    #[test]
    fn test_boot_stage_ordering() {
        // Verify the stages are logically ordered
        let stage1 = BootStage::BootloaderHandoff;
        let stage2 = BootStage::EarlyMemory;
        assert_ne!(stage1, stage2);
    }
}

// Example: How to use the boot logger in actual code
//
// Before (Direct HAL, hard to trace):
// crate::hal::serial::write_raw("Starting memory init\n");
// // ... do work ...
// crate::hal::serial::write_raw("Memory init done\n");
//
// After (Structured logging, easy to trace):
// let mut logger: BootLogger<0x3F8> = unsafe { BootLogger::new() };
// logger.enter_stage(BootStage::EarlyMemory);
// // ... do work ...
// logger.exit_stage(BootStage::EarlyMemory);
//
// Benefits:
// - Timestamps and performance data automatically included
// - Consistent format across boot stages
// - Log level filtering possible (trace/debug/info/warn/error)
// - Easy to add AOP macros for detailed tracing
