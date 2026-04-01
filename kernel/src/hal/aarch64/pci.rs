use crate::generated_consts::{
    AARCH64_PCI_ECAM_BASES, AARCH64_PCI_MAX_BUS, AARCH64_PCI_MAX_DEVICE, AARCH64_PCI_MAX_FUNCTION,
    AARCH64_PCI_SCAN_STOP_ON_FIRST_HIT,
};
use alloc::vec::Vec;
use core::ptr::read_volatile;

#[derive(Debug, Clone, Copy)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub ecam_base: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_core: u8,
    pub subclass: u8,
    pub header_type: u8,
    pub interrupt_line: u8,
    pub address: PciAddress,
}

impl PciAddress {
    fn offset(&self, reg: u16) -> Option<u64> {
        let mut off = self.ecam_base.checked_add((self.bus as u64) << 20)?;
        off = off.checked_add((self.device as u64) << 15)?;
        off = off.checked_add((self.function as u64) << 12)?;
        off.checked_add(reg as u64)
    }

    pub fn read_u32(&self, offset: u16) -> u32 {
        let Some(hhdm) = crate::hal::aarch64::hhdm_offset() else {
            return 0xFFFFFFFF;
        };
        let Some(phys) = self.offset(offset & !3) else {
            return 0xFFFFFFFF;
        };
        let Some(virt) = phys.checked_add(hhdm) else {
            return 0xFFFFFFFF;
        };
        let ptr = virt as *const u32;
        unsafe { read_volatile(ptr) }
    }

    pub fn read_vendor_id(&self) -> u16 {
        (self.read_u32(0) & 0xFFFF) as u16
    }

    pub fn read_device_id(&self) -> u16 {
        (self.read_u32(0) >> 16) as u16
    }

    pub fn read_header_type(&self) -> u8 {
        ((self.read_u32(0x0C) >> 16) & 0xFF) as u8
    }

    pub fn read_class_subclass(&self) -> (u8, u8) {
        let val = self.read_u32(0x08);
        (((val >> 24) & 0xFF) as u8, ((val >> 16) & 0xFF) as u8)
    }

    pub fn read_interrupt_line(&self) -> u8 {
        (self.read_u32(0x3C) & 0xFF) as u8
    }

    pub fn read_bar0(&self) -> u32 {
        self.read_u32(0x10)
    }
}

pub fn scan_bus() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    let mut found_ecam = false;
    let max_bus = AARCH64_PCI_MAX_BUS.min(255) as u8;
    let max_device = AARCH64_PCI_MAX_DEVICE.min(31);
    let max_function = AARCH64_PCI_MAX_FUNCTION.min(7);

    for &ecam_base in &AARCH64_PCI_ECAM_BASES {
        // Probe bus 0, device 0 to see if ECAM is here
        let probe_addr = PciAddress {
            bus: 0,
            device: 0,
            function: 0,
            ecam_base,
        };
        if probe_addr.read_vendor_id() == 0xFFFF {
            continue;
        }

        found_ecam = true;
        crate::klog_info!("Discovered PCI ECAM base at {:#x}", ecam_base);

        for bus in 0..=max_bus {
            for device in 0..=max_device {
                let addr = PciAddress {
                    bus,
                    device,
                    function: 0,
                    ecam_base,
                };
                let vendor_id = addr.read_vendor_id();

                if vendor_id == 0xFFFF {
                    continue;
                } // Device doesn't exist

                let header_type = addr.read_header_type();
                check_function(addr, &mut devices);

                if (header_type & 0x80) != 0 {
                    // Multi-function device
                    for function in 1..=max_function {
                        let func_addr = PciAddress {
                            bus,
                            device,
                            function,
                            ecam_base,
                        };
                        if func_addr.read_vendor_id() != 0xFFFF {
                            check_function(func_addr, &mut devices);
                        }
                    }
                }
            }
        }
        if AARCH64_PCI_SCAN_STOP_ON_FIRST_HIT {
            break; // Stop looking after we find the first valid ECAM
        }
    }

    if !found_ecam {
        crate::klog_warn!("No valid PCI ECAM base found in configured AArch64 list");
    }

    devices
}

fn check_function(addr: PciAddress, devices: &mut Vec<PciDevice>) {
    let vendor_id = addr.read_vendor_id();
    let device_id = addr.read_device_id();
    let (class, subclass) = addr.read_class_subclass();
    let header_type = addr.read_header_type();

    devices.push(PciDevice {
        address: addr,
        vendor_id,
        device_id,
        class_core: class,
        subclass,
        header_type,
        interrupt_line: addr.read_interrupt_line(),
    });
}
