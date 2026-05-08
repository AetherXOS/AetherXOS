/// Platform abstraction interfaces.
/// 
/// Traits for platform-specific services and capabilities,
/// allowing architecture-agnostic code to interact with hardware.

use crate::interfaces::KernelResult;
use alloc::string::String;

/// CPU feature flags
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub has_apic: bool,
    pub has_tsc: bool,
    pub has_msr: bool,
    pub has_paging: bool,
    pub has_interrupts: bool,
    pub has_virtualization: bool,
    pub has_protection: bool,
    pub supports_smp: bool,
    pub supports_virtualization: bool,
    pub cpu_count: u32,
    pub cpu_freq_mhz: u32,
}

/// Memory layout information
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub total_memory: usize,
    pub usable_memory: usize,
    pub physical_memory_size: u64,
    pub direct_map_base: u64,
    pub virt_bias: u64,
    pub reserved_start: usize,
    pub reserved_end: usize,
}

/// Platform capabilities
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub architecture: String,
    pub platform_name: String,
    pub cpu_count: u64,
    pub has_smp: bool,
    pub has_virtualization: bool,
    pub has_smm: bool,
    pub has_nested_paging: bool,
    pub supports_cpuid: bool,
    pub cpu_features: CpuFeatures,
    pub memory_layout: MemoryLayout,
    pub has_acpi: bool,
    pub has_device_tree: bool,
    pub max_interrupts: u32,
}

/// Trait for platform-specific services
pub trait PlatformServices {
    /// Get platform capabilities
    fn capabilities(&self) -> PlatformCapabilities;

    /// Get platform memory layout
    fn memory_layout(&self) -> MemoryLayout;

    /// Get platform CPU features
    fn cpu_features(&self) -> CpuFeatures;

    /// Get current CPU ID
    fn current_cpu_id(&self) -> u32;

    /// Get total CPU count
    fn cpu_count(&self) -> u32;

    /// Halt a specific CPU
    fn halt_cpu(&self, cpu_id: u32);

    /// Reset the entire platform
    fn reset_platform(&self, cold_reset: bool);

    /// Shutdown the platform
    fn shutdown(&self);

    /// Get platform time in nanoseconds since boot
    fn current_time_ns(&self) -> u64;

    /// Get platform cycle counter (arch-specific)
    fn cycle_count(&self) -> u64;

    /// Encode a runtime initialization trampoline (arch-specific)
    fn encode_init_trampoline(&self, buf: &mut [u8], hooks: &[u64], final_entry: u64) -> Option<usize>;

    /// Encode a runtime finalization trampoline (arch-specific)
    fn encode_fini_trampoline(&self, buf: &mut [u8], hooks: &[u64]) -> Option<usize>;

    /// Enable hardware interrupts
    fn enable_interrupts(&self);

    /// Disable hardware interrupts
    fn disable_interrupts(&self);

    /// Check if interrupts are enabled
    fn interrupts_enabled(&self) -> bool;

    /// Flush Translation Lookaside Buffer (TLB)
    fn flush_tlb(&self, addr: Option<u64>);

    /// Set current page table (CR3 on x86, TTBR0 on ARM)
    fn set_page_table(&self, root_phys_addr: u64);
}

/// Trait for complete platform abstraction
pub trait Platform: PlatformServices {
    /// Initialize the platform during boot
    fn init(&self) -> KernelResult<()>;

    /// Check if platform is ready
    fn is_ready(&self) -> bool;

    /// Platform-specific shutdown
    fn shutdown_platform(&self) -> KernelResult<()>;
    
    /// Get the platform services provider
    fn services(&self) -> &dyn PlatformServices;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_layout_validity() {
        let layout = MemoryLayout {
            kernel_base: 0,
            kernel_size: 0,
            total_memory: 1024 * 1024 * 1024, // 1GB
            usable_memory: 512 * 1024 * 1024,  // 512MB
            physical_memory_size: 0,
            direct_map_base: 0,
            virt_bias: 0,
            reserved_start: 0,
            reserved_end: 4096,
        };
        
        assert!(layout.total_memory >= layout.usable_memory);
        assert!(layout.reserved_end > layout.reserved_start);
    }

    #[test]
    fn test_cpu_features() {
        let features = CpuFeatures {
            has_apic: true,
            has_tsc: true,
            has_msr: true,
            has_paging: true,
            has_interrupts: true,
            has_virtualization: false,
            has_protection: true,
            supports_smp: true,
            supports_virtualization: true,
            cpu_count: 4,
            cpu_freq_mhz: 3000,
        };
        
        assert!(features.has_paging);
        assert!(features.cpu_count > 0);
    }
}
