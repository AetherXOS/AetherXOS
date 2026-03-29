pub(crate) fn log_dtb_discovery() {
    if let Some(dtb) = hypercore::hal::dtb_addr() {
        hypercore::klog_info!("DTB discovered at {:#x}", dtb);
    } else {
        hypercore::klog_debug!("DTB not provided by bootloader");
    }
}

pub(crate) fn init_acpi_discovery(enabled: bool) {
    if enabled {
        let topology = hypercore::hal::acpi::discover_topology();
        let power_info = hypercore::hal::acpi::discover_power_info();
        hypercore::kernel::power::init_from_acpi(power_info.has_fadt, power_info.fadt_revision);
        if topology.rsdp_addr != 0 {
            hypercore::klog_info!(
                "ACPI topology: rsdp={:#x} cpus={} ioapics={} isos={}",
                topology.rsdp_addr,
                topology.lapic_count,
                topology.ioapic_count,
                topology.iso_count
            );
        } else {
            hypercore::klog_warn!("ACPI RSDP not provided by bootloader");
        }
    } else {
        hypercore::klog_info!("ACPI discovery disabled by config");
    }
}

pub(crate) fn init_iommu_discovery(enabled: bool) {
    if enabled {
        hypercore::hal::iommu::init_platform_iommu();
        let iommu = hypercore::hal::iommu::stats();
        hypercore::klog_info!(
            "IOMMU state: backend={} hw_mode={} vtd_units={} vtd_programmed={} vtd_ready={} vtd_inv={} amdvi_units={} amdvi_inv={} amdvi_g={} amdvi_d={} amdvi_dev={} amdvi_fallback={} amdvi_timeout={} domains={} devices={} maps={} flushes={}",
            iommu.backend,
            iommu.hardware_mode,
            iommu.vtd_units,
            iommu.vtd_programmed_units,
            iommu.vtd_hw_ready,
            iommu.vtd_iotlb_inv_count,
            iommu.amdvi_units,
            iommu.amdvi_inv_count,
            iommu.amdvi_inv_global_count,
            iommu.amdvi_inv_domain_count,
            iommu.amdvi_inv_device_count,
            iommu.amdvi_inv_fallback_count,
            iommu.amdvi_inv_timeout_count,
            iommu.domains,
            iommu.attached_devices,
            iommu.mapping_count,
            iommu.flush_count
        );
    } else {
        hypercore::klog_info!("IOMMU initialization disabled by config");
    }
}
