#[derive(Debug, Clone, Copy)]
pub(super) struct OperationSupportSnapshot {
    pub control_plane_ready: bool,
    pub exit_tracing_ready: bool,
    pub interrupt_virtualization_ready: bool,
    pub time_virtualization_ready: bool,
    pub monitoring_ready: bool,
    pub resume_ready: bool,
    pub guest_entry_ready: bool,
    pub state_save_restore_ready: bool,
    pub trap_handling_ready: bool,
    pub device_passthrough_ready: bool,
    pub nested_ready: bool,
    pub summary: crate::hal::common::virt::VirtOperationSummary,
    pub snapshot_ready: bool,
    pub dirty_logging_ready: bool,
    pub live_migration_ready: bool,
    pub advanced_operations_tier: &'static str,
}

#[inline(always)]
pub(super) fn current_operation_support(
    virt: crate::hal::common::virt::VirtStatus,
    iommu: crate::hal::iommu::IommuStats,
    memory_isolation_ready: bool,
) -> OperationSupportSnapshot {
    let effective_policy = crate::config::KernelConfig::virtualization_effective_profile();
    let trap_tracing_enabled = crate::config::KernelConfig::virtualization_trap_tracing_enabled();
    let control_plane_ready =
        virt.vm_launch_ready && crate::hal::common::virt::hardware_accel_ready(virt);
    let interrupt_virtualization_ready = crate::hal::common::virt::hardware_accel_ready(virt)
        && (virt.vmx_vmcs_ready || virt.svm_vmcb_ready);
    let time_virtualization_ready = effective_policy.time_virtualization
        && crate::hal::common::virt::hardware_accel_ready(virt);
    let monitoring_ready = control_plane_ready && interrupt_virtualization_ready;
    let resume_ready = virt.vm_launch_ready && (virt.vmx_vmcs_ready || virt.svm_vmcb_ready);
    let guest_entry_ready = crate::hal::common::virt::hardware_accel_ready(virt) && resume_ready;
    let state_save_restore_ready = crate::hal::common::virt::hardware_accel_ready(virt)
        && (virt.enabled.vmxon_active || virt.enabled.svm_enabled);
    let trap_handling_ready = interrupt_virtualization_ready && state_save_restore_ready;
    let device_passthrough_ready =
        effective_policy.device_passthrough && memory_isolation_ready && iommu.attached_devices > 0;
    let exit_tracing_ready = trap_tracing_enabled && (virt.vmx_vmcs_ready || virt.svm_vmcb_ready);
    let summary = crate::hal::common::virt::summarize_operations(
        crate::hal::common::virt::VirtOperationFlags {
            control_plane_ready,
            exit_tracing_ready,
            interrupt_virtualization_ready,
            time_virtualization_ready,
            monitoring_ready,
            resume_ready,
            guest_entry_ready,
            state_save_restore_ready,
            trap_handling_ready,
            memory_isolation_ready,
            device_passthrough_ready,
        },
    );
    let snapshot_ready = effective_policy.snapshot && summary.snapshot_ready;
    let dirty_logging_ready = effective_policy.dirty_logging && summary.dirty_logging_ready;
    let live_migration_ready = effective_policy.live_migration && summary.live_migration_ready;
    let advanced_operations_tier = if snapshot_ready || dirty_logging_ready || live_migration_ready
    {
        summary.advanced_operations_tier
    } else {
        "disabled"
    };

    OperationSupportSnapshot {
        control_plane_ready,
        exit_tracing_ready,
        interrupt_virtualization_ready,
        time_virtualization_ready,
        monitoring_ready,
        resume_ready,
        guest_entry_ready,
        state_save_restore_ready,
        trap_handling_ready,
        device_passthrough_ready,
        nested_ready: effective_policy.nested
            && virt.vm_launch_ready
            && !virt.caps.hypervisor_present,
        summary,
        snapshot_ready,
        dirty_logging_ready,
        live_migration_ready,
        advanced_operations_tier,
    }
}
