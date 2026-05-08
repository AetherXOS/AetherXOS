/// Boot and initialization interfaces for the kernel.
/// 
/// This module defines traits for boot stages, subsystem initialization,
/// and runtime bootstrap that form the core of the architecture.

use crate::interfaces::KernelResult;
use core::fmt;

/// Represents a boot stage in kernel initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BootStage {
    /// Bootloader handoff, minimal setup
    BootloaderHandoff = 0,
    /// Early memory initialization (paging, heap)
    EarlyMemory = 1,
    /// CPU feature detection
    CpuFeatures = 2,
    /// Platform-specific early initialization
    PlatformEarly = 3,
    /// Interrupt/exception handlers installed
    HandlersReady = 4,
    /// Platform devices enumerated and initialized
    PlatformDevices = 5,
    /// Core subsystems (VFS, IPC, security) ready
    CoreSubsystems = 6,
    /// Interrupt window opened
    InterruptWindow = 7,
    /// Runtime fully ready
    RuntimeReady = 8,
    /// Userspace runtime prepared
    UserspaceReady = 9,
}

impl fmt::Display for BootStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Generic Result type for boot operations.
pub type BootResult<T = ()> = core::result::Result<T, &'static str>;

/// Trait for subsystems that need initialization during boot.
pub trait BootSubsystem: Send + Sync {
    /// Name of this subsystem (e.g., "VFS", "Scheduler", "IPC")
    fn name(&self) -> &'static str;

    /// Boot stage this subsystem needs to be initialized at
    fn required_stage(&self) -> BootStage;

    /// List of subsystem names this depends on (must be initialized first)
    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    /// Initialize this subsystem.
    fn init(&self) -> KernelResult<()>;

    /// Check if subsystem is ready for use
    fn is_ready(&self) -> bool;

    /// Optional shutdown/cleanup
    fn shutdown(&self) -> KernelResult<()> {
        Ok(())
    }
}

/// Trait for boot phase managers that coordinate initialization.
pub trait BootManager {
    /// Register a subsystem for initialization
    fn register_subsystem(&self, stage: BootStage, subsystem: &'static dyn BootSubsystem);

    /// Enter a boot stage
    fn enter_stage(&self, stage: BootStage) -> KernelResult<()>;

    /// Get current boot stage
    fn current_stage(&self) -> BootStage;

    /// Get boot diagnostics
    fn diagnostics(&self) -> BootDiagnostics;

    /// Get boot information snapshot
    fn boot_info(&self) -> BootInfo;

    /// Check if all critical subsystems are ready
    fn are_subsystems_ready(&self) -> bool;
}

/// Boot-time diagnostics and telemetry
#[derive(Debug, Clone, Copy)]
pub struct BootDiagnostics {
    pub stage_timings: [u64; 10],
    pub stage_errors: u32,
    pub warnings: u32,
}

/// Platform-specific boot information from bootloader
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    pub entry_stage: BootStage,
    pub current_stage: BootStage,
    pub subsystems_ready: u32,
    pub total_init_time_us: u64,
    pub boot_timestamp_us: u64,
    
    // Original fields kept for compatibility
    pub memory_size: usize,
    pub memory_start: usize,
    pub cpu_count: usize,
    pub cpu_freq_mhz: u32,
    pub platform_id: u32,
    pub acpi_rsdp: Option<usize>,
    pub dtb_address: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_stage_ordering() {
        assert!(BootStage::BootloaderHandoff < BootStage::EarlyMemory);
        assert!(BootStage::EarlyMemory < BootStage::CpuFeatures);
        assert!(BootStage::CoreSubsystems < BootStage::UserspaceReady);
    }
}
