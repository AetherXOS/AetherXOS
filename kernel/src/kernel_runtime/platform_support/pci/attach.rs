use super::super::config::PciAttachTelemetryConfig;

pub(crate) fn attach_pci_to_iommu_domain(
    enabled: bool,
    telemetry: PciAttachTelemetryConfig,
    devices: &[aethercore::hal::pci::PciDevice],
) {
    if !enabled {
        return;
    }

    let domain_id = 1u16;
    if !aethercore::hal::iommu::ensure_domain(domain_id) {
        aethercore::klog_warn!("IOMMU domain {} could not be created", domain_id);
        return;
    }

    let mut attached = 0usize;
    for dev in devices {
        let addr = aethercore::hal::iommu::DeviceAddress {
            bus: dev.address.bus,
            device: dev.address.device,
            function: dev.address.function,
        };
        if aethercore::hal::iommu::attach_device_to_domain(addr, domain_id) {
            attached = attached.saturating_add(1);
        }
    }

    if let Some((mappings, attached_devices, slpt_entries)) =
        aethercore::hal::iommu::domain_stats(domain_id)
    {
        aethercore::klog_info!(
            "IOMMU domain {} attached={} domain_devices={} domain_mappings={} slpt_entries={}",
            domain_id,
            attached,
            attached_devices,
            mappings,
            slpt_entries
        );
    }

    if super::super::should_log_security_telemetry(telemetry) {
        super::super::log_security_telemetry();
    }

    if super::super::should_log_ipc_telemetry(telemetry) {
        super::super::log_ipc_telemetry();
    }

    #[cfg(feature = "networking")]
    {
        if super::super::should_log_network_transport(telemetry) {
            super::super::log_network_transport_telemetry();
        }
    }
}
