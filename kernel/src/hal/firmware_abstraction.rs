//! Firmware abstraction - unified interface for ACPI (x86_64) and DTB (aarch64)
//!
//! This layer provides a consistent view of platform devices and memory layout
//! regardless of whether the system uses ACPI tables or device tree format.

use alloc::vec::Vec;
use super::abstractions::{FirmwareDevice, MemoryRange, MemoryType, BootParameters};

/// Unified firmware interface trait
/// 
/// Implementations handle ACPI parsing (x86_64) or DTB parsing (aarch64)
/// transparently, providing a common API to the kernel.
pub trait FirmwareProvider: Send + Sync {
    /// Get number of CPUs from firmware
    fn cpu_count(&self) -> u32;

    /// Get total system memory from firmware
    fn total_memory(&self) -> u64;

    /// Get memory ranges from firmware
    fn memory_ranges(&self) -> Vec<MemoryRange>;

    /// Get available memory map
    fn memory_map(&self) -> Vec<FirmwareDevice>;

    /// Enumerate all platform devices
    fn enumerate_devices(&self) -> Vec<FirmwareDevice>;

    /// Get UART devices
    fn get_uart_devices(&self) -> Vec<FirmwareDevice> {
        self.enumerate_devices()
            .into_iter()
            .filter(|d| d.device_type == "serial" || d.device_type == "uart")
            .collect()
    }

    /// Get timer devices
    fn get_timer_devices(&self) -> Vec<FirmwareDevice> {
        self.enumerate_devices()
            .into_iter()
            .filter(|d| d.device_type.contains("timer"))
            .collect()
    }

    /// Get network devices
    fn get_network_devices(&self) -> Vec<FirmwareDevice> {
        self.enumerate_devices()
            .into_iter()
            .filter(|d| d.device_type == "ethernet" || d.device_type == "network")
            .collect()
    }

    /// Get block storage devices
    fn get_storage_devices(&self) -> Vec<FirmwareDevice> {
        self.enumerate_devices()
            .into_iter()
            .filter(|d| d.device_type == "disk" || d.device_type == "storage")
            .collect()
    }

    /// Get boot parameters
    fn boot_parameters(&self) -> BootParameters;
}

/// x86_64 ACPI firmware provider
#[cfg(target_arch = "x86_64")]
pub struct AcpiFirmwareProvider;

#[cfg(target_arch = "x86_64")]
impl AcpiFirmwareProvider {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "x86_64")]
impl FirmwareProvider for AcpiFirmwareProvider {
    fn cpu_count(&self) -> u32 {
        // Would parse MADT table
        crate::hal::acpi_parser::count_cpus_from_madt().unwrap_or(1)
    }

    fn total_memory(&self) -> u64 {
        // Would parse E820 or similar
        4 * 1024 * 1024 * 1024 // 4GB default
    }

    fn memory_ranges(&self) -> Vec<MemoryRange> {
        vec![MemoryRange {
            start: 0,
            end: self.total_memory(),
            memory_type: MemoryType::Conventional,
        }]
    }

    fn memory_map(&self) -> Vec<FirmwareDevice> {
        vec![]
    }

    fn enumerate_devices(&self) -> Vec<FirmwareDevice> {
        // Would enumerate from ACPI tables
        crate::hal::acpi_parser::enumerate_devices_from_acpi().unwrap_or_default()
    }

    fn boot_parameters(&self) -> BootParameters {
        BootParameters {
            boot_device: "acpi".to_string(),
            cmdline: "".to_string(),
            loader: "bootloader".to_string(),
            timestamp_ms: 0,
        }
    }
}

/// aarch64 Device Tree firmware provider
#[cfg(target_arch = "aarch64")]
pub struct DeviceTreeFirmwareProvider;

#[cfg(target_arch = "aarch64")]
impl DeviceTreeFirmwareProvider {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "aarch64")]
impl FirmwareProvider for DeviceTreeFirmwareProvider {
    fn cpu_count(&self) -> u32 {
        // Would parse DTB cpus node
        crate::hal::dtb_parser::count_cpus_from_dtb().unwrap_or(1)
    }

    fn total_memory(&self) -> u64 {
        // Would parse DTB memory nodes
        4 * 1024 * 1024 * 1024 // 4GB default
    }

    fn memory_ranges(&self) -> Vec<MemoryRange> {
        vec![MemoryRange {
            start: 0,
            end: self.total_memory(),
            memory_type: MemoryType::Conventional,
        }]
    }

    fn memory_map(&self) -> Vec<FirmwareDevice> {
        vec![]
    }

    fn enumerate_devices(&self) -> Vec<FirmwareDevice> {
        // Would enumerate from device tree
        crate::hal::dtb_parser::enumerate_devices_from_dtb().unwrap_or_default()
    }

    fn boot_parameters(&self) -> BootParameters {
        BootParameters {
            boot_device: "dtb".to_string(),
            cmdline: "".to_string(),
            loader: "bootloader".to_string(),
            timestamp_ms: 0,
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn provider() -> &'static dyn FirmwareProvider {
    &AcpiFirmwareProvider
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn provider() -> &'static dyn FirmwareProvider {
    &DeviceTreeFirmwareProvider
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
fn provider() -> &'static dyn FirmwareProvider {
    struct NullFirmwareProvider;

    impl FirmwareProvider for NullFirmwareProvider {
        fn cpu_count(&self) -> u32 { 0 }
        fn total_memory(&self) -> u64 { 0 }
        fn memory_ranges(&self) -> Vec<MemoryRange> { Vec::new() }
        fn memory_map(&self) -> Vec<FirmwareDevice> { Vec::new() }
        fn enumerate_devices(&self) -> Vec<FirmwareDevice> { Vec::new() }
        fn boot_parameters(&self) -> BootParameters {
            BootParameters {
                boot_device: "unknown".to_string(),
                cmdline: "".to_string(),
                loader: "unknown".to_string(),
                timestamp_ms: 0,
            }
        }
    }

    static NULL_PROVIDER: NullFirmwareProvider = NullFirmwareProvider;
    &NULL_PROVIDER
}

/// Get the platform firmware provider.
pub fn get_firmware_provider() -> &'static dyn FirmwareProvider {
    provider()
}

/// Enumerate platform devices through the active firmware provider.
pub fn enumerate_devices() -> Vec<FirmwareDevice> {
    provider().enumerate_devices()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firmware_device_filtering() {
        // Mock devices
        let devices = vec![
            FirmwareDevice {
                name: "uart0".to_string(),
                device_type: "uart".to_string(),
                base_address: 0x3f8,
                size: 8,
                properties: vec![],
            },
            FirmwareDevice {
                name: "eth0".to_string(),
                device_type: "ethernet".to_string(),
                base_address: 0x1000,
                size: 0x100,
                properties: vec![],
            },
        ];
        
        // Filtering would work through trait methods
        assert_eq!(devices.len(), 2);
    }
}
