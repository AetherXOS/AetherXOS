use super::*;

#[path = "tests/policy.rs"]
mod policy;

fn base_x86_virt() -> crate::hal::common::virt::VirtStatus {
    crate::hal::common::virt::VirtStatus {
        caps: crate::hal::common::virt::VirtCaps {
            vmx: false,
            svm: false,
            hypervisor_present: false,
        },
        enabled: crate::hal::common::virt::VirtEnableState::default(),
        vm_launch_ready: false,
        blocker: "none",
        vmx_vmcs_ready: false,
        svm_vmcb_ready: false,
        prep_attempts: 0,
        prep_success: 0,
        prep_failures: 0,
        vmx_lifecycle: "uninitialized",
        svm_lifecycle: "uninitialized",
    }
}

fn empty_iommu() -> crate::hal::iommu::IommuStats {
    crate::hal::iommu::IommuStats {
        initialized: false,
        backend: "none",
        hardware_mode: false,
        vtd_units: 0,
        vtd_programmed_units: 0,
        vtd_hw_ready: false,
        vtd_iotlb_inv_count: 0,
        amdvi_units: 0,
        amdvi_inv_count: 0,
        amdvi_inv_global_count: 0,
        amdvi_inv_domain_count: 0,
        amdvi_inv_device_count: 0,
        amdvi_inv_fallback_count: 0,
        amdvi_inv_timeout_count: 0,
        domains: 0,
        attached_devices: 0,
        mapping_count: 0,
        flush_count: 0,
        map_count: 0,
        unmap_count: 0,
    }
}

#[test_case]
fn classify_prefers_virtual_machine_over_pc() {
    assert_eq!(classify_platform(true, true), PlatformKind::VirtualMachine);
    assert_eq!(classify_platform(true, false), PlatformKind::Pc);
    assert_eq!(
        classify_platform(false, false),
        PlatformKind::BareMetalUnknown
    );
}

#[test_case]
fn x86_virt_status_uses_passthrough_and_monitoring_rules() {
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
    assert!(status.monitoring_ready);
    assert!(status.resume_ready);
    assert!(status.guest_entry_ready);
    assert!(status.state_save_restore_ready);
    assert!(status.trap_handling_ready);
    assert_eq!(status.observability_tier, "full");
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "disabled");
    assert_eq!(status.backend_capability_level, "tier2");
    assert_eq!(status.backend_detail, "vmx:vmxon+vmcs");
    assert_eq!(status.capability_detail, "vmx:entry+vmcs+assist");
    assert_eq!(status.feature_detail, "vmx:entry-controls");
    assert_eq!(status.interrupt_detail, "vmx-posted-interrupt-ready");
    assert_eq!(status.time_detail, "vmx-tsc-offset-ready");
    assert_eq!(status.runtime_step, "hold-blocked-state");
    assert_eq!(status.runtime_selected_mode, "backend-blocked");
    assert_eq!(status.runtime_strategy, "conservative-hold");
    assert_eq!(status.runtime_budget_class, "minimal");
    assert_eq!(status.runtime_dispatch_class, "conservative");
    assert_eq!(status.runtime_preemption_policy, "hold");
    assert_eq!(status.runtime_scheduler_lane, "background");
    assert_eq!(status.runtime_dispatch_window, "hold-window");
    assert_eq!(status.runtime_execution_profile, "Background");
    assert_eq!(status.runtime_execution_profile_scope, "mixed-limits");
    assert_eq!(status.runtime_governor_profile, "Efficiency");
    assert_eq!(status.runtime_governor_profile_scope, "mixed-limits");
    assert_eq!(status.runtime_governor_class, "efficiency-governor");
    assert_eq!(status.runtime_latency_bias, "relaxed");
    assert_eq!(status.runtime_energy_bias, "saving");
    assert_eq!(status.runtime_aux_step, "no-aux-step");
    assert_eq!(
        status.runtime_blocked_by,
        Some("trap-dispatch-policy-disabled")
    );
    assert_eq!(status.policy_scope, "compiletime-limited");
    assert_eq!(status.control_detail, "vmx-control-active");
    assert_eq!(status.trap_detail, "vmx-traps-ready");
    assert_eq!(status.detect_state, "absent");
    assert_eq!(status.prepare_state, "prepared");
    assert_eq!(status.capability_state, "absent");
    assert_eq!(status.feature_state, "absent");
    assert_eq!(status.launch_state, "prepared");
    assert_eq!(status.resume_state, "prepared");
    assert_eq!(status.trap_state, "policy-limited");
    let lifecycle = status.lifecycle_snapshot();
    assert_eq!(lifecycle.summary, "prepared-policy-limited");
    assert_eq!(lifecycle.progress_per_mille, 328);
    assert_eq!(status.launch_stage, "guest-runnable");
    assert_eq!(status.isolation_tier, "dma-isolated");
    assert!(!status.device_passthrough_ready);
    assert_eq!(status.operational_readiness, "ready");
    assert!(status.can_launch_guest());
    assert!(status.can_resume_guest());
    assert!(!status.can_passthrough_devices());
    assert_eq!(status.operational_profile(), ("ready", true, false, false));
}

#[test_case]
fn x86_virt_status_marks_hardware_enabled_path_as_partial() {
    let mut virt = base_x86_virt();
    virt.caps.vmx = true;
    virt.enabled.vmx_enabled = true;
    virt.blocker = "vmx-awaiting-vmxon";
    virt.prep_attempts = 1;
    virt.vmx_lifecycle = "prepared";
    let iommu = empty_iommu();
    let status = virt_platform_status(virt, iommu, false);
    assert_eq!(status.backend_detail, "vmx:enabled");
    assert_eq!(status.backend_capability_level, "tier1");
    let lifecycle = status.lifecycle_snapshot();
    assert_eq!(lifecycle.summary, "prepared-policy-limited");
    assert_eq!(lifecycle.progress_per_mille, 328);
    assert_eq!(status.launch_stage, "hardware-enabled");
    assert_eq!(status.operational_readiness, "partial");
    assert!(!status.can_launch_guest());
    assert!(!status.can_resume_guest());
    assert!(!status.can_passthrough_devices());
    assert!(!status.can_enable_nested());
    assert!(!status.can_trace_exits());
    assert!(!status.can_virtualize_time());
    assert_eq!(
        status.operational_profile(),
        ("partial", false, false, false)
    );
}

#[test_case]
fn x86_virt_status_marks_launch_prepared_path_as_staged() {
    let mut virt = base_x86_virt();
    virt.caps.vmx = true;
    virt.enabled.vmx_enabled = true;
    virt.enabled.vmxon_active = true;
    virt.vm_launch_ready = true;
    virt.blocker = "vmcs-not-ready";
    virt.prep_attempts = 2;
    virt.prep_success = 1;
    virt.prep_failures = 1;
    virt.vmx_lifecycle = "prepared";
    let iommu = empty_iommu();
    let status = virt_platform_status(virt, iommu, true);
    assert_eq!(status.backend_detail, "vmx:enabled");
    assert_eq!(status.launch_stage, "launch-prepared");
    assert_eq!(status.operational_readiness, "staged");
    assert_eq!(status.observability_tier, "minimal");
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "disabled");
    assert!(!status.control_plane_ready);
    assert!(!status.can_launch_guest());
    assert!(!status.can_resume_guest());
    assert!(!status.can_enable_nested());
    assert!(!status.can_trace_exits());
    assert!(!status.can_virtualize_time());
    assert_eq!(
        status.operational_profile(),
        ("staged", false, false, false)
    );
}

#[test_case]
fn x86_svm_ready_path_reports_operational_profile() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(true));
    crate::config::KernelConfig::set_virtualization_dirty_logging_enabled(Some(true));

    let mut virt = base_x86_virt();
    virt.caps.svm = true;
    virt.enabled.svm_enabled = true;
    virt.vm_launch_ready = true;
    virt.svm_vmcb_ready = true;
    virt.prep_attempts = 3;
    virt.prep_success = 3;
    virt.svm_lifecycle = "active";
    let status = virt_platform_status(virt, empty_iommu(), false);
    assert_eq!(status.backend, "svm");
    assert_eq!(status.backend_detail, "svm:enabled+vmcb");
    assert_eq!(status.capability_detail, "svm:efer+vmcb");
    assert_eq!(status.feature_detail, "svm:control-enable");
    assert_eq!(status.interrupt_detail, "svm-exit-interrupt-ready");
    assert_eq!(status.time_detail, "svm-tsc-offset-ready");
    assert_eq!(status.operational_profile(), ("ready", true, false, false));
    assert_eq!(status.advanced_operations_tier, "disabled");

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn x86_monitoring_with_trap_handling_reaches_full_observability() {
    let mut virt = base_x86_virt();
    virt.caps.vmx = true;
    virt.enabled.vmx_enabled = true;
    virt.enabled.vmxon_active = true;
    virt.vm_launch_ready = true;
    virt.vmx_vmcs_ready = true;
    virt.prep_attempts = 2;
    virt.prep_success = 1;
    virt.prep_failures = 1;
    virt.vmx_lifecycle = "prepared";
    let status = virt_platform_status(virt, empty_iommu(), false);
    assert!(status.monitoring_ready);
    assert!(status.trap_handling_ready);
    assert_eq!(status.observability_tier, "full");
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "disabled");
}
