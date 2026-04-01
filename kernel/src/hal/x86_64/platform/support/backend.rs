#[derive(Debug, Clone, Copy)]
pub(super) struct BackendSupportSnapshot {
    pub execution_backend: &'static str,
    pub irq_backend: &'static str,
    pub memory_backend: &'static str,
    pub launch_path: &'static str,
}

#[inline(always)]
pub(super) fn current_backend_support(
    backend: &'static str,
    hardware_accel: bool,
    iommu: crate::hal::iommu::IommuStats,
    x2apic_enabled: bool,
) -> BackendSupportSnapshot {
    let execution_backend = match backend {
        "vmx" => "vmx-root",
        "svm" => "svm-host",
        "el2" => "el2",
        _ => "none",
    };
    let irq_backend = if x2apic_enabled { "x2apic" } else { "apic" };
    let memory_backend = if iommu.initialized && iommu.hardware_mode {
        iommu.backend
    } else if hardware_accel {
        "shadow-paging"
    } else {
        "none"
    };
    let launch_path = match backend {
        "vmx" if hardware_accel => "vmxon-vmlaunch",
        "svm" if hardware_accel => "svm-vmrun",
        _ => "none",
    };

    BackendSupportSnapshot {
        execution_backend,
        irq_backend,
        memory_backend,
        launch_path,
    }
}
