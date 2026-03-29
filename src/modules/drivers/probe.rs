use crate::hal::pci::PciDevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciId {
    pub vendor_id: u16,
    pub device_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciClassCode {
    pub class_core: u8,
    pub subclass: u8,
}

#[inline(always)]
pub const fn pci_id(vendor_id: u16, device_id: u16) -> PciId {
    PciId {
        vendor_id,
        device_id,
    }
}

#[inline(always)]
pub const fn pci_class(class_core: u8, subclass: u8) -> PciClassCode {
    PciClassCode {
        class_core,
        subclass,
    }
}

#[inline(always)]
pub fn probe_first_pci_by_ids(devices: &[PciDevice], ids: &[PciId]) -> Option<PciDevice> {
    for dev in devices {
        if device_matches_any_pci_id(dev, ids) {
            return Some(*dev);
        }
    }
    None
}

#[inline(always)]
pub fn probe_first_pci_by_class(devices: &[PciDevice], class: PciClassCode) -> Option<PciDevice> {
    for dev in devices {
        if device_matches_pci_class(dev, class) {
            return Some(*dev);
        }
    }
    None
}

#[inline(always)]
pub fn device_matches_any_pci_id(device: &PciDevice, ids: &[PciId]) -> bool {
    for id in ids {
        if device.vendor_id == id.vendor_id && device.device_id == id.device_id {
            return true;
        }
    }
    false
}

#[inline(always)]
pub fn device_matches_pci_class(device: &PciDevice, class: PciClassCode) -> bool {
    device.class_core == class.class_core && device.subclass == class.subclass
}

#[inline(always)]
pub fn pci_bar0_io_base(device: PciDevice) -> Option<u16> {
    let bar0 = device.address.read_bar0();
    if (bar0 & 1) == 0 {
        return None;
    }
    Some((bar0 & !0x3) as u16)
}

#[inline(always)]
pub fn pci_bar0_mmio_base(device: PciDevice) -> Option<u64> {
    let bar0 = device.address.read_bar0();
    if (bar0 & 1) != 0 {
        return None;
    }
    let base = (bar0 as u64) & !0xFu64;
    if base == 0 {
        return None;
    }
    Some(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::pci::{PciAddress, PciDevice};

    fn fake_device(vendor_id: u16, device_id: u16, class_core: u8, subclass: u8) -> PciDevice {
        PciDevice {
            address: PciAddress::new(0, 0, 0),
            vendor_id,
            device_id,
            class_core,
            subclass,
            header_type: 0,
            interrupt_line: 0,
        }
    }

    #[test_case]
    fn probe_helpers_match_by_id_and_class() {
        let devices = [
            fake_device(0x8086, 0x100e, 0x02, 0x00),
            fake_device(0x1af4, 0x1000, 0x02, 0x00),
            fake_device(0x8086, 0xabcd, 0x01, 0x08),
        ];
        let ids = [pci_id(0x1af4, 0x1000), pci_id(0x1af4, 0x1001)];
        let net = probe_first_pci_by_ids(&devices, &ids);
        assert!(net.is_some());
        let net = net.unwrap_or(fake_device(0, 0, 0, 0));
        assert_eq!(net.vendor_id, 0x1af4);
        assert_eq!(net.device_id, 0x1000);

        let class = probe_first_pci_by_class(&devices, pci_class(0x01, 0x08));
        assert!(class.is_some());
        let class = class.unwrap_or(fake_device(0, 0, 0, 0));
        assert_eq!(class.class_core, 0x01);
        assert_eq!(class.subclass, 0x08);
    }
}
