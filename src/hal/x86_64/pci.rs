use crate::hal::x86_64::port::X86PortIo;
use crate::interfaces::{PciController, PortIo};
use alloc::vec::Vec;
#[path = "pci_support.rs"]
mod pci_support;
use pci_support::{byte_shift, config_address, word_shift};

pub struct X86Pci;

impl PciController for X86Pci {
    #[inline(always)]
    unsafe fn read_config_byte(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        // Safety: the config address register was primed immediately above for this read.
        (unsafe { X86PortIo::ind(CONFIG_DATA) } >> byte_shift(offset)) as u8
    }

    #[inline(always)]
    unsafe fn read_config_word(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        // Safety: the config address register was primed immediately above for this read.
        (unsafe { X86PortIo::ind(CONFIG_DATA) } >> word_shift(offset)) as u16
    }

    #[inline(always)]
    unsafe fn read_config_dword(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        // Safety: the config address register was primed immediately above for this read.
        unsafe { X86PortIo::ind(CONFIG_DATA) }
    }

    #[inline(always)]
    unsafe fn write_config_byte(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u8) {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        let shift = byte_shift(offset);
        // Safety: the config address register was primed immediately above for this read-modify-write.
        let mut dword = unsafe { X86PortIo::ind(CONFIG_DATA) };
        dword = (dword & !(0xFF << shift)) | ((value as u32) << shift);
        // Safety: writing back to the data port completes the same config transaction.
        unsafe { X86PortIo::outd(CONFIG_DATA, dword) };
    }

    #[inline(always)]
    unsafe fn write_config_word(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u16) {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        let shift = word_shift(offset);
        // Safety: the config address register was primed immediately above for this read-modify-write.
        let mut dword = unsafe { X86PortIo::ind(CONFIG_DATA) };
        dword = (dword & !(0xFFFF << shift)) | ((value as u32) << shift);
        // Safety: writing back to the data port completes the same config transaction.
        unsafe { X86PortIo::outd(CONFIG_DATA, dword) };
    }

    #[inline(always)]
    unsafe fn write_config_dword(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
        let address = config_address(bus, slot, func, offset);
        // Safety: PCI config access uses the standard x86 config address/data ports.
        unsafe { X86PortIo::outd(CONFIG_ADDRESS, address) };
        // Safety: writing back to the data port completes the same config transaction.
        unsafe { X86PortIo::outd(CONFIG_DATA, value) };
    }
}

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub address: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_core: u8,
    pub subclass: u8,
    pub header_type: u8,
    pub interrupt_line: u8,
}

impl PciAddress {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
        }
    }

    fn read_u32(&self, offset: u8) -> u32 {
        let address = config_address(self.bus, self.device, self.function, offset);

        unsafe {
            // Safety: PCI config access uses the standard x86 config address/data ports.
            X86PortIo::outd(CONFIG_ADDRESS, address);
            // Safety: the config address register was primed immediately above for this read.
            X86PortIo::ind(CONFIG_DATA)
        }
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

    pub fn read_bar0(&self) -> u32 {
        self.read_u32(0x10)
    }

    pub fn read_interrupt_line(&self) -> u8 {
        (self.read_u32(0x3C) & 0xFF) as u8
    }
}

pub fn scan_bus() -> Vec<PciDevice> {
    let mut devices = Vec::new();

    for bus in 0..=255 {
        for device in 0..32 {
            let addr = PciAddress::new(bus, device, 0);
            let vendor_id = addr.read_vendor_id();

            if vendor_id == 0xFFFF {
                continue;
            } // Device doesn't exist

            let header_type = addr.read_header_type();
            check_function(addr, &mut devices);

            if (header_type & 0x80) != 0 {
                // Multi-function device
                for function in 1..8 {
                    let func_addr = PciAddress::new(bus, device, function);
                    if func_addr.read_vendor_id() != 0xFFFF {
                        check_function(func_addr, &mut devices);
                    }
                }
            }
        }
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

// Common Device Classes
pub const CLASS_MASS_STORAGE: u8 = 0x01;
pub const CLASS_NETWORK: u8 = 0x02;
pub const CLASS_DISPLAY: u8 = 0x03;

pub const VENDOR_INTEL: u16 = 0x8086;
pub const VENDOR_REDHAT: u16 = 0x1AF4; // VirtIO

pub const VIRTIO_DEV_BLK_LEGACY: u16 = 0x1001;
pub const VIRTIO_DEV_BLK_MODERN: u16 = 0x1042;
pub const VIRTIO_DEV_NET_LEGACY: u16 = 0x1000;
