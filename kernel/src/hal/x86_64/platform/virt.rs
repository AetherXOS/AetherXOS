use super::{PlatformKind, PlatformStatus};

pub(super) fn status(
    acpi_present: bool,
    dtb_present: bool,
    virt: crate::hal::common::virt::VirtStatus,
    iommu: crate::hal::iommu::IommuStats,
) -> PlatformStatus {
    let virt_status =
        super::support::virt_platform_status(virt, iommu, super::super::apic::is_x2apic());
    super::support::compose_platform_status(
        super::support::PlatformBaseStatus {
            kind: PlatformKind::VirtualMachine,
            acpi_present,
            dtb_present,
            hypervisor_present: virt.caps.hypervisor_present,
            iommu_ready: iommu.initialized && iommu.hardware_mode,
            iommu_backend: iommu.backend,
            cpu_count: super::super::smp::cpu_count(),
            ap_online: super::super::smp::ap_online_count(),
            x2apic_supported: super::super::apic::supports_x2apic(),
            x2apic_enabled: super::super::apic::is_x2apic(),
            vm_launch_ready: virt.vm_launch_ready,
        },
        virt_status,
    )
}
