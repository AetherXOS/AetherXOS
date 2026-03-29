use super::*;

#[path = "tests/policy.rs"]
mod policy;

fn base_el2_virt() -> crate::hal::common::virt::VirtStatus {
    crate::hal::common::virt::VirtStatus {
        caps: crate::hal::common::virt::VirtCaps {
            vmx: false,
            svm: false,
            hypervisor_present: true,
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

fn base_timer(freq: u64) -> crate::hal::aarch64::timer::GenericTimerStats {
    crate::hal::aarch64::timer::GenericTimerStats {
        frequency_hz: freq,
        last_programmed_ticks: 0,
        clamp_min_hits: 0,
        clamp_max_hits: 0,
    }
}

#[test_case]
fn classify_distinguishes_virt_server_and_unknown() {
    assert_eq!(classify_platform(false, true, true), PlatformKind::Virt);
    assert_eq!(classify_platform(true, false, false), PlatformKind::Server);
    assert_eq!(
        classify_platform(false, false, false),
        PlatformKind::BareMetalUnknown
    );
}

#[test_case]
fn aarch64_virt_status_tracks_monitoring_and_resume() {
    let mut virt = base_el2_virt();
    virt.vm_launch_ready = true;
    virt.prep_attempts = 1;
    virt.prep_success = 1;
    virt.vmx_lifecycle = "active";
    virt.svm_lifecycle = "active";
    let gic = crate::hal::aarch64::gic::GicStats {
        initialized: true,
        version: 3,
    };
    let timer = base_timer(1_000_000);
    let status = virt_platform_status(virt, gic, timer, true);
    assert!(status.monitoring_ready);
    assert!(status.resume_ready);
    assert!(status.guest_entry_ready);
    assert!(status.state_save_restore_ready);
    assert!(status.trap_handling_ready);
    assert_eq!(status.observability_tier, "full");
    assert!(status.snapshot_ready);
    assert!(status.dirty_logging_ready);
    assert!(status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "hypervisor-grade");
    assert_eq!(status.backend_capability_level, "tier3");
    assert_eq!(status.backend_detail, "el2:active+gic+timer");
    assert_eq!(status.capability_detail, "el2:timer+gic+entry");
    assert_eq!(status.feature_detail, "el2:vgic+vtimer");
    assert_eq!(status.interrupt_detail, "gicv3-virt-ready");
    assert_eq!(status.time_detail, "cntv-virtual-time-ready");
    assert_eq!(status.runtime_step, "resume-el2-guest");
    assert_eq!(status.runtime_selected_mode, "backend-full");
    assert_eq!(status.runtime_strategy, "stateful-balanced");
    assert_eq!(status.runtime_budget_class, "wide");
    assert_eq!(status.runtime_dispatch_class, "balanced");
    assert_eq!(status.runtime_preemption_policy, "preemptible");
    assert_eq!(status.runtime_scheduler_lane, "balanced");
    assert_eq!(status.runtime_dispatch_window, "adaptive-window");
    assert_eq!(status.runtime_execution_profile, "Balanced");
    assert_eq!(status.runtime_execution_profile_scope, "fully-enabled");
    assert_eq!(status.runtime_governor_profile, "Balanced");
    assert_eq!(status.runtime_governor_profile_scope, "fully-enabled");
    assert_eq!(status.runtime_governor_class, "balanced");
    assert_eq!(status.runtime_latency_bias, "balanced");
    assert_eq!(status.runtime_energy_bias, "balanced");
    assert_eq!(status.runtime_aux_step, "prepare-live-migration-state");
    assert_eq!(status.runtime_blocked_by, None);
    assert_eq!(status.policy_scope, "fully-enabled");
    assert_eq!(status.control_detail, "el2-control-active");
    assert_eq!(status.trap_detail, "el2-traps-ready");
    assert_eq!(status.detect_state, "detected");
    assert_eq!(status.prepare_state, "prepared");
    assert_eq!(status.capability_state, "active");
    assert_eq!(status.feature_state, "active");
    assert_eq!(status.launch_state, "prepared");
    assert_eq!(status.resume_state, "ready");
    assert_eq!(status.trap_state, "prepared");
    let lifecycle = status.lifecycle_snapshot();
    assert_eq!(lifecycle.summary, "trap-ready");
    assert_eq!(lifecycle.progress_per_mille, 1000);
    assert_eq!(status.launch_stage, "guest-runnable");
    assert_eq!(status.isolation_tier, "stage2-gicv3");
    assert!(status.device_passthrough_ready);
    assert_eq!(status.operational_readiness, "ready");
    assert!(status.can_launch_guest());
    assert!(status.can_resume_guest());
    assert!(status.can_passthrough_devices());
    assert_eq!(status.operational_profile(), ("ready", true, true, true));
}

#[test_case]
fn aarch64_virt_status_marks_launch_only_path_as_staged() {
    let mut virt = base_el2_virt();
    virt.vm_launch_ready = true;
    virt.blocker = "gic-not-ready";
    virt.prep_attempts = 1;
    virt.prep_success = 1;
    virt.vmx_lifecycle = "active";
    virt.svm_lifecycle = "active";
    let gic = crate::hal::aarch64::gic::GicStats {
        initialized: false,
        version: 0,
    };
    let timer = base_timer(1_000_000);
    let status = virt_platform_status(virt, gic, timer, false);
    assert_eq!(status.backend_detail, "el2:active");
    assert_eq!(status.backend_capability_level, "tier1");
    let lifecycle = status.lifecycle_snapshot();
    assert_eq!(lifecycle.summary, "capability-active");
    assert_eq!(lifecycle.progress_per_mille, 571);
    assert_eq!(status.launch_stage, "launch-prepared");
    assert_eq!(status.operational_readiness, "staged");
    assert_eq!(status.observability_tier, "minimal");
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "baseline");
    assert!(status.can_launch_guest());
    assert!(!status.can_resume_guest());
    assert!(!status.can_passthrough_devices());
    assert!(!status.can_enable_nested());
    assert!(!status.can_trace_exits());
    assert!(!status.can_virtualize_time());
    assert_eq!(
        status.operational_profile(),
        ("staged", false, false, false)
    );
}

#[test_case]
fn aarch64_virt_status_marks_detected_only_path_as_blocked() {
    let mut virt = base_el2_virt();
    virt.blocker = "EL2 Not Active";
    virt.prep_attempts = 1;
    virt.prep_failures = 1;
    virt.vmx_lifecycle = "failed";
    virt.svm_lifecycle = "failed";
    let gic = crate::hal::aarch64::gic::GicStats {
        initialized: false,
        version: 0,
    };
    let timer = base_timer(0);
    let status = virt_platform_status(virt, gic, timer, false);
    assert_eq!(status.backend_detail, "el2:detected");
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
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "baseline");
}

#[test_case]
fn aarch64_monitoring_without_trap_handling_is_partial_observability() {
    let mut virt = base_el2_virt();
    virt.vm_launch_ready = true;
    virt.prep_attempts = 1;
    virt.prep_success = 1;
    virt.vmx_lifecycle = "active";
    virt.svm_lifecycle = "active";
    let gic = crate::hal::aarch64::gic::GicStats {
        initialized: true,
        version: 3,
    };
    let timer = base_timer(0);
    let status = virt_platform_status(virt, gic, timer, true);
    assert!(status.monitoring_ready);
    assert!(!status.trap_handling_ready);
    assert_eq!(status.observability_tier, "partial");
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.advanced_operations_tier, "baseline");
}
