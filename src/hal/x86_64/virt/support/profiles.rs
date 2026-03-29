use super::*;

pub(crate) fn snapshot_ready() -> bool {
    let current = status();
    let monitoring_ready =
        current.vm_launch_ready && (current.vmx_vmcs_ready || current.svm_vmcb_ready);
    let state_save_restore_ready = (current.caps.vmx && current.enabled.vmxon_active)
        || (current.caps.svm && current.enabled.svm_enabled);
    crate::hal::common::virt::snapshot_ready_from_flags(state_save_restore_ready, monitoring_ready)
}

pub(crate) fn dirty_logging_ready() -> bool {
    let current = status();
    let trap_handling_ready =
        (current.caps.vmx && current.vmx_vmcs_ready && current.enabled.vmxon_active)
            || (current.caps.svm && current.svm_vmcb_ready && current.enabled.svm_enabled);
    let memory_isolation_ready =
        crate::hal::iommu::stats().initialized && crate::hal::iommu::stats().hardware_mode;
    crate::hal::common::virt::dirty_logging_ready_from_flags(
        trap_handling_ready,
        memory_isolation_ready,
    )
}

pub(crate) fn live_migration_ready() -> bool {
    let current = status();
    let hardware_time_ready =
        (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
            || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    crate::hal::common::virt::live_migration_ready_from_flags(
        snapshot_ready(),
        hardware_time_ready,
        dirty_logging_ready(),
    )
}

pub(crate) fn advanced_operations_profile() -> (bool, bool, bool, &'static str) {
    crate::hal::common::virt::advanced_operations_profile(
        snapshot_ready(),
        dirty_logging_ready(),
        live_migration_ready(),
    )
}

pub(crate) fn guest_lifecycle_profile() -> (&'static str, bool, bool, &'static str) {
    let current = status();
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    let advanced = advanced_operations_profile();
    crate::hal::common::virt::guest_lifecycle_profile(
        lifecycle,
        current.vm_launch_ready,
        crate::hal::common::virt::guest_resume_ready(
            lifecycle,
            current.vm_launch_ready,
            crate::hal::common::virt::has_launch_context(lifecycle),
        ),
        advanced.3,
    )
}

pub(crate) fn guest_control_profile() -> (&'static str, bool, bool, bool) {
    let current = status();
    let control_ready = (current.caps.vmx && current.enabled.vmx_enabled)
        || (current.caps.svm && current.enabled.svm_enabled);
    let trap_ready = (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
        || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    crate::hal::common::virt::guest_control_profile(
        control_ready,
        trap_ready,
        snapshot_ready(),
        current.vm_launch_ready,
    )
}

pub(crate) fn guest_runtime_profile() -> (&'static str, bool, bool, bool, bool) {
    let current = status();
    let control_ready = (current.caps.vmx && current.enabled.vmx_enabled)
        || (current.caps.svm && current.enabled.svm_enabled);
    let trap_ready = (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
        || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    crate::hal::common::virt::guest_runtime_profile(crate::hal::common::virt::GuestRuntimeFlags {
        launch_ready: current.vm_launch_ready,
        control_ready,
        trap_ready,
        resume_ready: crate::hal::common::virt::guest_resume_ready(
            lifecycle,
            current.vm_launch_ready,
            crate::hal::common::virt::has_launch_context(lifecycle),
        ),
        snapshot_ready: snapshot_ready(),
    })
}

pub(crate) fn guest_exit_profile() -> (&'static str, bool, bool, bool, bool) {
    let current = status();
    let trap_ready = (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
        || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    crate::hal::common::virt::guest_exit_profile(crate::hal::common::virt::GuestExitFlags {
        launch_ready: current.vm_launch_ready,
        trap_ready,
        trace_ready: trap_ready,
        interrupt_ready: trap_ready,
        time_ready: trap_ready,
    })
}

pub(crate) fn guest_launch_profile() -> (&'static str, bool, bool, bool) {
    let current = status();
    let control_ready = (current.caps.vmx && current.enabled.vmx_enabled)
        || (current.caps.svm && current.enabled.svm_enabled);
    let guest_entry_ready =
        (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
            || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    let memory_isolation_ready =
        crate::hal::iommu::stats().initialized && crate::hal::iommu::stats().hardware_mode;
    crate::hal::common::virt::guest_launch_profile(crate::hal::common::virt::GuestLaunchFlags {
        launch_ready: current.vm_launch_ready,
        control_ready,
        guest_entry_ready,
        memory_isolation_ready,
    })
}

pub(crate) fn guest_operation_profile() -> crate::hal::common::virt::GuestOperationProfile {
    let current = status();
    let control_ready = (current.caps.vmx && current.enabled.vmx_enabled)
        || (current.caps.svm && current.enabled.svm_enabled);
    let trap_ready = (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
        || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    let guest_entry_ready =
        (current.caps.vmx && current.enabled.vmxon_active && current.vmx_vmcs_ready)
            || (current.caps.svm && current.enabled.svm_enabled && current.svm_vmcb_ready);
    let memory_isolation_ready =
        crate::hal::iommu::stats().initialized && crate::hal::iommu::stats().hardware_mode;
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    crate::hal::common::virt::guest_operation_profile(
        crate::hal::common::virt::GuestLaunchFlags {
            launch_ready: current.vm_launch_ready,
            control_ready,
            guest_entry_ready,
            memory_isolation_ready,
        },
        crate::hal::common::virt::GuestRuntimeFlags {
            launch_ready: current.vm_launch_ready,
            control_ready,
            trap_ready,
            resume_ready: crate::hal::common::virt::guest_resume_ready(
                lifecycle,
                current.vm_launch_ready,
                crate::hal::common::virt::has_launch_context(lifecycle),
            ),
            snapshot_ready: snapshot_ready(),
        },
        crate::hal::common::virt::GuestExitFlags {
            launch_ready: current.vm_launch_ready,
            trap_ready,
            trace_ready: trap_ready,
            interrupt_ready: trap_ready,
            time_ready: guest_entry_ready,
        },
    )
}
