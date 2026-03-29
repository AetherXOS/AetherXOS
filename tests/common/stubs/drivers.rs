pub fn pci_read_config(_bus: u8, _device: u8, _function: u8, _offset: u8) -> u32 {
    0xFFFFFFFF
}

pub fn pci_write_config(_bus: u8, _device: u8, _function: u8, _offset: u8, _value: u32) {}

pub fn pci_find_device(_vendor_id: u16, _device_id: u16) -> Option<(u8, u8, u8)> {
    None
}

pub fn pci_enumerate() -> Vec<PciDevice> {
    Vec::new()
}

pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub bar: [u32; 6],
}

pub struct PciBar {
    pub address: u64,
    pub size: u64,
    pub is_io: bool,
    pub is_prefetchable: bool,
}

impl PciBar {
    pub fn new() -> Self {
        Self {
            address: 0,
            size: 0,
            is_io: false,
            is_prefetchable: false,
        }
    }
}

impl Default for PciBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PciDevice {
    fn default() -> Self {
        Self {
            bus: 0,
            device: 0,
            function: 0,
            vendor_id: 0,
            device_id: 0,
            class: 0,
            subclass: 0,
            prog_if: 0,
            bar: [0; 6],
        }
    }
}
