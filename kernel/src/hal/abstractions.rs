//! HAL Abstraction Layer - Core Traits
//!
//! This module defines unified abstractions for all hardware components.
//! All architecture-specific implementations (x86_64, aarch64) must implement these traits.
//! This ensures NO architecture-specific code leaks outside the hal/ module.

use alloc::vec::Vec;
use alloc::string::String;

/// Platform identifier for runtime feature detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformType {
    /// x86_64 architecture
    X86_64,
    /// ARM64 (aarch64) architecture
    AArch64,
    /// Unsupported platform
    Unknown,
}

impl PlatformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::AArch64 => "aarch64",
            Self::Unknown => "unknown",
        }
    }
}

/// CPU feature flags - unified across architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatures {
    /// SIMD support (SSE/AVX on x86_64, NEON on aarch64)
    pub simd: bool,
    /// Virtual machine support (VMX/SVM on x86_64, VHE/NV on aarch64)
    pub virtualization: bool,
    /// Hardware cryptography (AES-NI, SHA on x86_64; AES, SHA on aarch64)
    pub crypto: bool,
    /// Performance counters available
    pub perf_counters: bool,
    /// Memory tagging extension (MTE on aarch64)
    pub memory_tagging: bool,
    /// Pointer authentication (PAC on aarch64)
    pub pointer_auth: bool,
    /// Branch prediction control
    pub bp_control: bool,
    /// Number of performant CPU cores (physical)
    pub num_physical_cores: u32,
    /// Number of logical CPUs (with hyperthreading)
    pub num_logical_cpus: u32,
    /// Maximum addressable memory (bytes)
    pub max_memory: u64,
}

impl CpuFeatures {
    pub fn new() -> Self {
        Self {
            simd: false,
            virtualization: false,
            crypto: false,
            perf_counters: false,
            memory_tagging: false,
            pointer_auth: false,
            bp_control: false,
            num_physical_cores: 1,
            num_logical_cpus: 1,
            max_memory: 4 * 1024 * 1024 * 1024, // 4GB default
        }
    }
}

/// Default physical base for simple PMM/bootstrap allocators.
pub const fn default_pmm_base() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        0x10_0000usize // 1 MiB
    }

    #[cfg(target_arch = "aarch64")]
    {
        0x4000_0000usize // typical AArch64 virt DRAM base
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        0usize
    }
}

/// Interrupt model - how platform handles interrupts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptModel {
    /// x86_64: PIC or APIC model
    Pic,
    Apic,
    /// aarch64: GIC (Generic Interrupt Controller)
    Gic,
    /// Custom/unknown
    Custom,
}

/// Timer model - how platform provides timing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerModel {
    /// x86_64: Programmable Interval Timer (PIT)
    Pit,
    /// x86_64: APIC Timer
    ApicTimer,
    /// x86_64: TSC-based timing
    Tsc,
    /// aarch64: ARM Generic Timer
    ArmTimer,
    /// HPET (High Precision Event Timer)
    Hpet,
    /// Custom/unknown
    Custom,
}

/// CPU context representation - platform-specific register set
#[derive(Debug, Clone)]
pub enum CpuContext {
    /// x86_64 context
    X86_64(X86_64Context),
    /// aarch64 context
    AArch64(AArch64Context),
}

/// x86_64-specific CPU context
#[derive(Debug, Clone, Default)]
pub struct X86_64Context {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr3: u64,
}

/// aarch64-specific CPU context
#[derive(Debug, Clone, Default)]
pub struct AArch64Context {
    pub x: [u64; 31], // x0-x30
    pub sp: u64,
    pub pc: u64,
    pub pstate: u64,
    pub ttbr0_el1: u64,
}

impl Default for CpuContext {
    fn default() -> Self {
        CpuContext::X86_64(X86_64Context::default())
    }
}

/// Platform initialization result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitResult {
    /// Initialization successful
    Success,
    /// Feature not available
    Unavailable,
    /// Hardware error
    Error,
    /// Partial initialization (some features unavailable)
    Partial,
}

/// Core platform abstraction trait
/// 
/// All platform implementations must provide these capabilities.
/// This allows generic kernel code to work across architectures.
pub trait PlatformAbstraction: Send + Sync {
    /// Get the platform type
    fn platform_type(&self) -> PlatformType;

    /// Get CPU features
    fn cpu_features(&self) -> &CpuFeatures;

    /// Get interrupt model for this platform
    fn interrupt_model(&self) -> InterruptModel;

    /// Get timer model for this platform
    fn timer_model(&self) -> TimerModel;

    /// Early platform initialization (before virtual memory)
    fn early_init(&self) -> InitResult;

    /// Late platform initialization (after virtual memory)
    fn late_init(&self) -> InitResult;

    /// Initialize interrupts
    fn init_interrupts(&self) -> InitResult;

    /// Initialize timer
    fn init_timer(&self) -> InitResult;

    /// Initialize multi-core support
    fn init_smp(&self) -> InitResult;

    /// Get number of available CPUs
    fn cpu_count(&self) -> u32 {
        self.cpu_features().num_logical_cpus
    }

    /// Check if CPU supports a specific feature
    fn supports_simd(&self) -> bool {
        self.cpu_features().simd
    }

    fn supports_virtualization(&self) -> bool {
        self.cpu_features().virtualization
    }

    fn supports_crypto(&self) -> bool {
        self.cpu_features().crypto
    }

    /// Get current time in nanoseconds since boot
    fn get_time_ns(&self) -> u64;

    /// Set IRQ handler for given interrupt number
    fn set_irq_handler(&self, irq: u32, handler: extern "C" fn()) -> InitResult;

    /// Enable/disable interrupts
    fn enable_interrupts(&self);
    fn disable_interrupts(&self);
    fn irq_save(&self) -> usize;
    fn irq_restore(&self, flags: usize);

    /// Halt CPU
    fn halt(&self) -> !;

    /// Idle until next interrupt
    fn idle_once(&self);
}

/// Device tree / Firmware abstraction
/// Provides unified interface to platform device information
pub trait FirmwareInterface: Send + Sync {
    /// Get CPUID for the platform
    fn cpuid(&self) -> u32;

    /// Get total system memory size
    fn total_memory(&self) -> u64;

    /// Get available memory ranges
    fn memory_ranges(&self) -> Vec<MemoryRange>;

    /// Get memory map entry
    fn get_memory_map(&self) -> Vec<MemoryMapEntry>;

    /// Enumerate devices from firmware
    fn enumerate_devices(&self) -> Vec<FirmwareDevice>;

    /// Get boot parameters
    fn boot_parameters(&self) -> BootParameters;
}

/// Memory range descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryRange {
    pub start: u64,
    pub end: u64,
    pub memory_type: MemoryType,
}

/// Memory type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Usable RAM
    Conventional,
    /// Reserved for firmware/BIOS/UEFI
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS (non-volatile storage)
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Persistent memory
    Persistent,
    /// Unknown
    Other,
}

/// Memory map entry
#[derive(Debug, Clone)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub entry_type: u32,
}

/// Firmware device descriptor
#[derive(Debug, Clone)]
pub struct FirmwareDevice {
    pub name: String,
    pub device_type: String,
    pub base_address: u64,
    pub size: u64,
    pub properties: Vec<(String, String)>,
}

/// Boot parameters passed to kernel
#[derive(Debug, Clone)]
pub struct BootParameters {
    pub boot_device: String,
    pub cmdline: String,
    pub loader: String,
    pub timestamp_ms: u64,
}

/// CPU abstraction trait
pub trait CpuAbstraction: Send + Sync {
    /// Get current CPU ID
    fn current_cpu_id(&self) -> u32;

    /// Get CPU context (current state of registers)
    fn get_context(&self) -> CpuContext;

    /// Set CPU context (restore registers)
    fn set_context(&self, context: CpuContext);

    /// Check if CPU has feature
    fn has_feature(&self, feature: &str) -> bool;

    /// Enter low-power state
    fn sleep(&self, duration_ms: u64);

    /// Number of CPU cores
    fn core_count(&self) -> u32;
}

/// IRQ controller abstraction
pub trait IrqController: Send + Sync {
    /// Register IRQ handler
    fn register_handler(&self, irq: u32, handler: extern "C" fn()) -> InitResult;

    /// Enable IRQ
    fn enable_irq(&self, irq: u32) -> InitResult;

    /// Disable IRQ
    fn disable_irq(&self, irq: u32) -> InitResult;

    /// Clear/acknowledge IRQ
    fn clear_irq(&self, irq: u32) -> InitResult;

    /// Get pending IRQs
    fn get_pending_irqs(&self) -> u64;
}

/// Timer controller abstraction
pub trait TimerController: Send + Sync {
    /// Set timer for given milliseconds
    fn set_timer(&self, millis: u64) -> InitResult;

    /// Get current timer count
    fn get_timer(&self) -> u64;

    /// Clear/stop timer
    fn clear_timer(&self) -> InitResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_type_string() {
        assert_eq!(PlatformType::X86_64.as_str(), "x86_64");
        assert_eq!(PlatformType::AArch64.as_str(), "aarch64");
    }

    #[test]
    fn test_cpu_features_default() {
        let features = CpuFeatures::new();
        assert!(!features.simd);
        assert_eq!(features.num_logical_cpus, 1);
        assert_eq!(features.max_memory, 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_memory_type_classification() {
        let mt = MemoryType::Conventional;
        assert_eq!(mt, MemoryType::Conventional);
    }

    #[test]
    fn test_interrupt_models() {
        let _pic = InterruptModel::Pic;
        let _apic = InterruptModel::Apic;
        let _gic = InterruptModel::Gic;
        // All should be distinct
    }

    #[test]
    fn test_timer_models() {
        let _pit = TimerModel::Pit;
        let _apic = TimerModel::ApicTimer;
        let _arm = TimerModel::ArmTimer;
    }
}
