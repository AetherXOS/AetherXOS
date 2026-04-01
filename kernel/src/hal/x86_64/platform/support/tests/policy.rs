use super::*;

#[test_case]
fn x86_policy_can_disable_advanced_virtualization_operations() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_snapshot_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_dirty_logging_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_live_migration_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_nested_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_time_virtualization_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_device_passthrough_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_execution_policy_profile(Some(
        crate::config::VirtualizationExecutionProfile {
            scheduling_class: crate::config::VirtualizationExecutionClass::Background,
        },
    ));

    let virt = crate::hal::common::virt::VirtStatus {
        caps: crate::hal::common::virt::VirtCaps {
            vmx: true,
            svm: false,
            hypervisor_present: false,
        },
        enabled: crate::hal::common::virt::VirtEnableState {
            vmx_enabled: true,
            vmxon_active: true,
            svm_enabled: false,
        },
        vm_launch_ready: true,
        blocker: "none",
        vmx_vmcs_ready: true,
        svm_vmcb_ready: false,
        prep_attempts: 2,
        prep_success: 2,
        prep_failures: 0,
        vmx_lifecycle: "active",
        svm_lifecycle: "uninitialized",
    };
    let iommu = crate::hal::iommu::IommuStats {
        initialized: true,
        backend: "vtd",
        hardware_mode: true,
        vtd_units: 1,
        vtd_programmed_units: 1,
        vtd_hw_ready: true,
        vtd_iotlb_inv_count: 0,
        amdvi_units: 0,
        amdvi_inv_count: 0,
        amdvi_inv_global_count: 0,
        amdvi_inv_domain_count: 0,
        amdvi_inv_device_count: 0,
        amdvi_inv_fallback_count: 0,
        amdvi_inv_timeout_count: 0,
        domains: 1,
        attached_devices: 2,
        mapping_count: 1,
        flush_count: 0,
        map_count: 0,
        unmap_count: 0,
    };
    let status = virt_platform_status(virt, iommu, true);
    assert!(!status.exit_tracing_ready);
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.runtime_step, "resume-vmx-vcpu-basic");
    assert_eq!(status.runtime_selected_mode, "backend-blocked");
    assert_eq!(status.runtime_strategy, "conservative-hold");
    assert_eq!(status.runtime_budget_class, "minimal");
    assert_eq!(status.runtime_dispatch_class, "conservative");
    assert_eq!(status.runtime_preemption_policy, "hold");
    assert_eq!(status.runtime_scheduler_lane, "background");
    assert_eq!(status.runtime_dispatch_window, "hold-window");
    assert_eq!(status.runtime_execution_profile, "Background");
    assert_eq!(status.runtime_execution_profile_scope, "runtime-limited");
    assert_eq!(status.runtime_governor_profile, "Balanced");
    assert_eq!(status.runtime_governor_profile_scope, "fully-enabled");
    assert_eq!(status.runtime_governor_class, "background-optimized");
    assert_eq!(status.runtime_latency_bias, "relaxed");
    assert_eq!(status.runtime_energy_bias, "saving");
    assert_eq!(status.runtime_aux_step, "no-aux-step");
    assert_eq!(status.policy_scope, "runtime-limited");
    assert_eq!(status.advanced_operations_tier, "disabled");
    assert!(!status.nested_ready);
    assert!(!status.time_virtualization_ready);
    assert!(!status.device_passthrough_ready);

    crate::config::KernelConfig::reset_runtime_overrides();
}
