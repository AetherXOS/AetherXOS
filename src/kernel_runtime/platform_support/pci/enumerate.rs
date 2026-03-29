pub(crate) fn enumerate_pci(enabled: bool) -> alloc::vec::Vec<hypercore::hal::pci::PciDevice> {
    use hypercore::hal::pci;

    let devices = if enabled {
        pci::scan_bus()
    } else {
        alloc::vec::Vec::new()
    };

    if enabled {
        hypercore::klog_info!("PCI found {} devices", devices.len());
    } else {
        hypercore::klog_info!("PCI enumeration disabled by config");
    }

    for dev in &devices {
        hypercore::klog_debug!(
            "PCI {:02x}:{:02x}.{:x} Vendor={:04x} Dev={:04x} Class={:02x}/{:02x}",
            dev.address.bus,
            dev.address.device,
            dev.address.function,
            dev.vendor_id,
            dev.device_id,
            dev.class_core,
            dev.subclass
        );
    }

    devices
}
