use super::platform_support::{
    attach_pci_to_iommu_domain, enumerate_pci, init_acpi_discovery, init_iommu_discovery,
    init_smp_runtime, init_virtualization_bootstrap, log_dtb_discovery,
    run_platform_runtime_orchestration, PciAttachTelemetryConfig, PlatformTelemetryConfig,
};
use crate::kernel_runtime::KernelRuntime;
use aethercore::generated_consts::{
    CORE_ENABLE_ACPI_DISCOVERY, CORE_ENABLE_IOMMU, CORE_ENABLE_PCI_ENUMERATION, CORE_ENABLE_SMP,
    CORE_ENABLE_VIRTUALIZATION,
};

impl KernelRuntime {
    pub(super) fn init_platform_services(&self) {
        let telemetry = PlatformTelemetryConfig::collect();

        log_dtb_discovery();
        init_acpi_discovery(CORE_ENABLE_ACPI_DISCOVERY);
        init_iommu_discovery(CORE_ENABLE_IOMMU);
        init_virtualization_bootstrap(CORE_ENABLE_VIRTUALIZATION);
        run_platform_runtime_orchestration(telemetry);
    }

    pub(super) fn attach_pci_to_iommu_domain(&self, devices: &[aethercore::hal::pci::PciDevice]) {
        let telemetry = PciAttachTelemetryConfig::collect();
        attach_pci_to_iommu_domain(CORE_ENABLE_IOMMU, telemetry, devices);
    }

    pub(super) fn enumerate_pci(&self) -> alloc::vec::Vec<aethercore::hal::pci::PciDevice> {
        enumerate_pci(CORE_ENABLE_PCI_ENUMERATION)
    }

    pub(super) fn init_smp(&self) {
        init_smp_runtime(CORE_ENABLE_SMP);
    }
}
