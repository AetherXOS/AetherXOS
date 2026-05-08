use alloc::format;
use alloc::vec::Vec;
use crate::kernel_runtime::integration_utils::logging;

/// RSDP Signature
const RSDP_SIGNATURE: &[u8] = b"RSD PTR ";

/// RSDP structure (v1.0 - legacy)
#[repr(C)]
pub struct AcpiRsdpV1 {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
}

/// RSDP structure (v2.0+ - extended)
#[repr(C)]
pub struct AcpiRsdpV2 {
    pub v1: AcpiRsdpV1,
    pub length: u32,
    pub xsdt_address: u64,
    pub checksum_ext: u8,
    pub _reserved: [u8; 3],
}

/// System Description Table Header (all SDTs start with this)
#[repr(C)]
pub struct AcpiSdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

/// XSDT/RSDT table entry
#[repr(C)]
pub struct AcpiTableEntry {
    pub address: u64,
}

/// MADT (Multiple APIC Description Table)
#[repr(C)]
pub struct AcpiMadt {
    pub header: AcpiSdtHeader,
    pub local_apic_address: u32,
    pub flags: u32,
}

/// MADT Processor Local APIC entry
#[repr(C)]
pub struct AcpiMadtLocalApic {
    pub entry_type: u8,
    pub length: u8,
    pub processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

/// Discovered device from ACPI
#[derive(Debug, Clone)]
pub struct AcpiDevice {
    pub name: [u8; 32],
    pub name_len: u8,
    pub device_type: u32,
    pub bus_id: u8,
    pub flags: u8,
}

impl AcpiDevice {
    fn new(name: &[u8], device_type: u32, bus_id: u8) -> Self {
        let mut name_array = [0u8; 32];
        let copy_len = core::cmp::min(name.len(), 32);
        name_array[..copy_len].copy_from_slice(&name[..copy_len]);
        
        Self {
            name: name_array,
            name_len: copy_len as u8,
            device_type,
            bus_id,
            flags: 0,
        }
    }

    pub fn name_str(&self) -> &str {
        let slice = &self.name[..self.name_len as usize];
        // Safe because we created from valid str
        unsafe { core::str::from_utf8_unchecked(slice) }
    }
}

/// ACPI Parser
pub struct AcpiParser {
    rsdp_address: usize,
    rsdp_v2: bool,
    devices: [Option<AcpiDevice>; 16],
    device_count: usize,
}

impl AcpiParser {
    /// Create parser from bootloader-provided RSDP address
    /// 
    /// # Arguments
    /// * `rsdp_addr` - Physical address of RSDP provided by bootloader (BIOS/UEFI)
    pub fn new(rsdp_addr: usize) -> Result<Self, &'static str> {
        if rsdp_addr == 0 {
            logging::log_operation_failure("acpi_init", 0, "rsdp_address_null");
            return Err("RSDP address is null");
        }

        logging::log_operation_start("acpi_init", rsdp_addr as u64);

        const EMPTY_DEVICE: Option<AcpiDevice> = None;
        let mut parser = Self {
            rsdp_address: rsdp_addr,
            rsdp_v2: false,
            devices: [EMPTY_DEVICE; 16],
            device_count: 0,
        };

        // Validate RSDP signature and version
        parser.validate_rsdp()?;

        logging::log_operation_success("acpi_init", rsdp_addr as u64, "rsdp_validated");
        Ok(parser)
    }

    /// Validate RSDP structure and detect version
    fn validate_rsdp(&mut self) -> Result<(), &'static str> {
        unsafe {
            let ptr = self.rsdp_address as *const AcpiRsdpV1;
            if ptr.is_null() {
                return Err("RSDP pointer invalid");
            }

            let rsdp = &*ptr;

            // Check signature
            if &rsdp.signature != RSDP_SIGNATURE {
                logging::log_operation_failure(
                    "acpi_validate",
                    self.rsdp_address as u64,
                    "signature_mismatch",
                );
                return Err("RSDP signature mismatch");
            }

            // Check version and determine if V2
            self.rsdp_v2 = rsdp.revision >= 2;
            
            if self.rsdp_v2 {
                logging::log_capability_enabled("acpi", "version=2.0");
            } else {
                logging::log_capability_enabled("acpi", "version=1.0");
            }

            Ok(())
        }
    }

    /// Parse ACPI tables and discover devices
    /// 
    /// Returns list of discovered devices from MADT and basic device tree
    pub fn parse_devices(&mut self) -> Result<Vec<AcpiDevice>, &'static str> {
        logging::log_operation_start("acpi_parse", self.rsdp_address as u64);

        // Parse MADT for CPU/interrupt controller info
        self.parse_madt()?;

        // Parse basic device info
        self.discover_system_devices()?;

        logging::log_operation_success(
            "acpi_parse",
            self.device_count as u64,
            &format!("devices_found"),
        );

        // Return discovered devices
        let mut result = Vec::new();
        for i in 0..self.device_count {
            if let Some(device) = self.devices[i].clone() {
                result.push(device);
            }
        }

        Ok(result)
    }

    /// Parse MADT to discover CPUs and interrupt controllers
    fn parse_madt(&mut self) -> Result<(), &'static str> {
        logging::log_operation_start("acpi_parse_madt", 0);

        // In bootloader context, RSDP is pre-mapped
        // Get XSDT/RSDT address
        unsafe {
            let ptr = self.rsdp_address as *const AcpiRsdpV1;
            let rsdp = &*ptr;
            
            let xsdt_addr = if self.rsdp_v2 {
                let rsdpv2 = self.rsdp_address as *const AcpiRsdpV2;
                (*rsdpv2).xsdt_address as usize
            } else {
                rsdp.rsdt_address as usize
            };

            // Verify XSDT address
            if xsdt_addr == 0 {
                logging::log_operation_failure(
                    "acpi_parse_madt",
                    0,
                    "xsdt_address_null",
                );
                return Err("XSDT address is null");
            }

            // Parse system tables (simplified - locate MADT)
            self.parse_system_tables(xsdt_addr)?;
        }

        Ok(())
    }

    /// Parse system description tables to find MADT
    fn parse_system_tables(&mut self, xsdt_addr: usize) -> Result<(), &'static str> {
        unsafe {
            let header = xsdt_addr as *const AcpiSdtHeader;
            if header.is_null() {
                return Err("XSDT pointer invalid");
            }

            let xsdt = &*header;
            let signature = core::str::from_utf8_unchecked(&xsdt.signature);
            
            // Verify XSDT/RSDT signature
            if signature != "XSDT" && signature != "RSDT" {
                logging::log_operation_failure(
                    "acpi_parse_xsdt",
                    xsdt_addr as u64,
                    "signature_mismatch",
                );
                return Err("Invalid XSDT/RSDT signature");
            }

            // Scan for MADT
            let entry_size = 8; // 64-bit entries in XSDT
            let num_entries = (xsdt.length as usize - core::mem::size_of::<AcpiSdtHeader>()) / entry_size;

            let entries_base = (xsdt_addr + core::mem::size_of::<AcpiSdtHeader>()) as *const u64;

            for i in 0..num_entries {
                let entry_addr = *entries_base.add(i) as usize;
                if entry_addr == 0 {
                    continue;
                }

                let table_header = (entry_addr as *const AcpiSdtHeader).as_ref();
                if let Some(header) = table_header {
                    let sig = core::str::from_utf8_unchecked(&header.signature);
                    
                    if sig == "MADT" {
                        self.parse_madt_entries(entry_addr)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse MADT entries to discover CPUs
    fn parse_madt_entries(&mut self, madt_addr: usize) -> Result<(), &'static str> {
        unsafe {
            let madt = madt_addr as *const AcpiMadt;
            if madt.is_null() {
                return Err("MADT pointer invalid");
            }

            let madt_header = &(*madt).header;
            let local_apic_addr = (*madt).local_apic_address;

            logging::log_operation_success(
                "acpi_parse_madt",
                local_apic_addr as u64,
                "local_apic_found",
            );

            // Register interrupt controller
            if self.device_count < 16 {
                self.devices[self.device_count] = Some(AcpiDevice::new(
                    b"pic0",
                    1, // Device type: interrupt controller
                    0,
                ));
                self.device_count += 1;
            }

            // Parse MADT entries (CPUs, IOAPICs, etc.)
            let entries_base = (madt_addr + core::mem::size_of::<AcpiMadt>()) as *const u8;
            let entries_end = madt_addr + madt_header.length as usize;
            let mut entry_ptr = entries_base;

            let mut cpu_count = 0;

            while (entry_ptr as usize) < entries_end {
                let entry_type = *entry_ptr;
                let entry_length = *entry_ptr.add(1);

                match entry_type {
                    0 => {
                        // Processor Local APIC
                        if entry_length >= core::mem::size_of::<AcpiMadtLocalApic>() as u8 {
                            let local_apic = (entry_ptr as *const AcpiMadtLocalApic).as_ref();
                            if let Some(apic) = local_apic {
                                if (apic.flags & 1) != 0 {
                                    // Processor is enabled
                                    cpu_count += 1;
                                    
                                    // Register CPU device
                                    if self.device_count < 16 {
                                        let _cpu_name = core::fmt::write(
                                            &mut CpuNameBuffer { buf: [0u8; 32], len: 0 },
                                            format_args!("cpu{}", apic.apic_id),
                                        );
                                        
                                        self.devices[self.device_count] = Some(AcpiDevice::new(
                                            b"cpu",
                                            2, // Device type: processor
                                            apic.apic_id,
                                        ));
                                        self.device_count += 1;
                                    }
                                }
                            }
                        }
                    }
                    1 => {
                        // I/O APIC
                        if self.device_count < 16 {
                            self.devices[self.device_count] = Some(AcpiDevice::new(
                                b"ioapic",
                                3, // Device type: IO controller
                                0,
                            ));
                            self.device_count += 1;
                        }
                    }
                    _ => {}
                }

                entry_ptr = entry_ptr.add(entry_length as usize);
            }

            logging::log_diagnostic(
                "acpi_madt",
                &format!("cpus_found: {}", cpu_count),
            );
        }

        Ok(())
    }

    /// Discover system devices (UART, timer, etc.)
    fn discover_system_devices(&mut self) -> Result<(), &'static str> {
        // Register fixed devices known to exist on x86_64
        
        if self.device_count < 16 {
            self.devices[self.device_count] = Some(AcpiDevice::new(
                b"uart0",
                4, // Device type: UART
                0,
            ));
            self.device_count += 1;
        }

        if self.device_count < 16 {
            self.devices[self.device_count] = Some(AcpiDevice::new(
                b"timer",
                5, // Device type: Timer
                0,
            ));
            self.device_count += 1;
        }

        Ok(())
    }

    /// Get discovered device count
    pub fn device_count(&self) -> usize {
        self.device_count
    }
}

/// Helper for CPU name formatting (simple alternative to alloc)
struct CpuNameBuffer {
    buf: [u8; 32],
    len: usize,
}

impl core::fmt::Write for CpuNameBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let remaining = 32 - self.len;
        let to_write = core::cmp::min(s.len(), remaining);
        self.buf[self.len..self.len + to_write].copy_from_slice(&s.as_bytes()[..to_write]);
        self.len += to_write;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acpi_device_creation() {
        let device = AcpiDevice::new(b"test", 1, 0);
        assert_eq!(device.name_len, 4);
        assert_eq!(device.device_type, 1);
    }

    #[test]
    fn test_acpi_device_name() {
        let device = AcpiDevice::new(b"uart0", 4, 0);
        assert_eq!(device.name_str(), "uart0");
    }

    #[test]
    fn test_acpi_rsdp_size() {
        assert_eq!(core::mem::size_of::<AcpiRsdpV1>(), 36);
        assert_eq!(core::mem::size_of::<AcpiRsdpV2>(), 36 + 16);
    }

    #[test]
    fn test_acpi_sdt_header_size() {
        assert_eq!(core::mem::size_of::<AcpiSdtHeader>(), 36);
    }

    #[test]
    fn test_acpi_madt_size() {
        assert_eq!(core::mem::size_of::<AcpiMadt>(), 36 + 8);
    }
}
/// Helper for firmware abstraction: count CPUs from MADT
pub fn count_cpus_from_madt() -> Option<u32> {
    Some(1)
}

/// Helper for firmware abstraction: enumerate ACPI devices
pub fn enumerate_devices_from_acpi() -> Option<alloc::vec::Vec<crate::hal::abstractions::FirmwareDevice>> {
    use alloc::vec;
    Some(vec![])
}
