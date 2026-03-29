use super::{
    activate_lifecycle_code, advanced_operations_profile, advanced_operations_tier, backend_family,
    backend_has_full_runtime, backend_name, backend_operational_path, backend_runtime_step,
    can_enable_nested_from_flags, can_launch_from_readiness, can_passthrough_from_flags,
    can_resume_from_flags, can_trace_from_flags, can_virtualize_time_from_flags,
    capability_has_entry_path, control_is_operational, dirty_logging_ready_from_flags,
    execute_guest_entry, execute_guest_resume, execute_guest_trap, guest_backend_execution,
    guest_backend_runtime_plan, guest_backend_state_machine, guest_control_profile,
    guest_entry_intent, guest_entry_operation, guest_exit_hook, guest_exit_profile,
    guest_launch_hook, guest_launch_profile, guest_lifecycle_profile, guest_next_runtime_action,
    guest_operation_bundle, guest_operation_decision, guest_operation_hooks, guest_operation_plan,
    guest_operation_profile, guest_orchestration_summary, guest_resume_intent,
    guest_resume_operation, guest_resume_ready, guest_runtime_dispatch_hint,
    guest_runtime_execution, guest_runtime_hook, guest_runtime_profile,
    guest_runtime_scheduling_profile, guest_transition_state, guest_trap_intent,
    guest_trap_operation, hardware_accel_ready, has_launch_context, interrupt_is_operational,
    live_migration_ready_from_flags, observability_tier_from_flags,
    operational_readiness_from_stage, operational_smoke_profile, prep_success_rate_per_mille,
    primary_lifecycle, reset_lifecycle_code, smoke_profile, snapshot_ready_from_flags,
    summarize_operations, teardown_lifecycle_code, time_is_operational, trap_is_operational,
    virtualization_runtime_governor, GuestBackendExecution, GuestBackendRuntimePlan,
    GuestBackendStateMachine, GuestEntryIntent, GuestExecutionStep, GuestExitFlags, GuestExitHook,
    GuestLaunchFlags, GuestLaunchHook, GuestOperationBundle, GuestOperationDecision,
    GuestOperationHooks, GuestOperationPlan, GuestResumeIntent, GuestRuntimeDispatchHint,
    GuestRuntimeExecution, GuestRuntimeFlags, GuestRuntimeHook, GuestTransitionState,
    GuestTrapIntent, VirtCaps, VirtEnableState, VirtOperationFlags, VirtStatus,
    VirtualizationPowerTuning, VirtualizationRebalanceTuning, VirtualizationSchedulerTuning,
    BACKEND_EL2_FULL, BACKEND_VMX_ACTIVE, BACKEND_VMX_ENABLED, CAPABILITY_EL2_ENABLED,
    CAPABILITY_VMX_ACTIVE, CAPABILITY_VMX_DETECTED, CONTROL_DETECTED, CONTROL_EL2_ACTIVE,
    CONTROL_NONE, INTERRUPT_BASIC, INTERRUPT_VMX_READY, LIFECYCLE_CODE_ACTIVE,
    LIFECYCLE_CODE_PREPARED, LIFECYCLE_CODE_TORN_DOWN, LIFECYCLE_CODE_UNINITIALIZED,
    LIFECYCLE_STATE_ACTIVE, LIFECYCLE_STATE_FAILED, LIFECYCLE_STATE_PREPARED, TIME_BASIC,
    TIME_VMX_READY, TRAP_NOT_READY, TRAP_VMX_READY,
};

#[path = "tests/hooks.rs"]
mod hooks;
#[path = "tests/policy.rs"]
mod policy;

fn base_status() -> VirtStatus {
    VirtStatus {
        caps: VirtCaps::default(),
        enabled: VirtEnableState::default(),
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

#[test_case]
fn operational_readiness_maps_expected_stages() {
    assert_eq!(
        operational_readiness_from_stage("guest-runnable", "tier3"),
        "ready"
    );
    assert_eq!(
        operational_readiness_from_stage("launch-prepared", "tier1"),
        "staged"
    );
    assert_eq!(
        operational_readiness_from_stage("hardware-enabled", "tier1"),
        "partial"
    );
    assert_eq!(
        operational_readiness_from_stage("unavailable", "tier0"),
        "blocked"
    );
}

#[test_case]
fn can_launch_only_for_ready_and_staged() {
    assert!(can_launch_from_readiness("ready"));
    assert!(can_launch_from_readiness("staged"));
    assert!(!can_launch_from_readiness("partial"));
    assert!(!can_launch_from_readiness("blocked"));
}

#[test_case]
fn backend_name_and_hardware_accel_cover_vmx_svm_and_el2() {
    let mut vmx = base_status();
    vmx.caps.vmx = true;
    vmx.enabled.vmx_enabled = true;
    vmx.enabled.vmxon_active = true;
    vmx.vmx_vmcs_ready = true;
    assert_eq!(backend_name(vmx), "vmx");
    assert!(hardware_accel_ready(vmx));

    let mut svm = base_status();
    svm.caps.svm = true;
    svm.enabled.svm_enabled = true;
    svm.svm_vmcb_ready = true;
    assert_eq!(backend_name(svm), "svm");
    assert!(hardware_accel_ready(svm));

    let mut el2 = base_status();
    el2.caps.hypervisor_present = true;
    el2.vm_launch_ready = true;
    assert_eq!(backend_name(el2), "el2");
    assert!(hardware_accel_ready(el2));
}

#[test_case]
fn guest_resume_ready_requires_launch_context_and_prepared_lifecycle() {
    assert!(guest_resume_ready("prepared", true, true));
    assert!(guest_resume_ready("active", true, true));
    assert!(!guest_resume_ready("failed", true, true));
    assert!(!guest_resume_ready("prepared", true, false));
    assert!(!guest_resume_ready("prepared", false, true));
}

#[test_case]
fn prep_rate_and_smoke_profile_cover_common_backend_paths() {
    let mut vmx = base_status();
    vmx.caps.vmx = true;
    vmx.enabled.vmx_enabled = true;
    vmx.enabled.vmxon_active = true;
    vmx.vmx_vmcs_ready = true;
    vmx.prep_attempts = 4;
    vmx.prep_success = 3;
    assert_eq!(prep_success_rate_per_mille(vmx), 750);
    assert_eq!(
        smoke_profile(vmx, "guest-runnable", "tier3"),
        ("vmx", true, 750, "ready", true)
    );

    let mut svm = base_status();
    svm.caps.svm = true;
    svm.enabled.svm_enabled = true;
    svm.prep_attempts = 2;
    svm.prep_success = 1;
    assert_eq!(
        smoke_profile(svm, "hardware-enabled", "tier1"),
        ("svm", false, 500, "partial", false)
    );

    let mut el2 = base_status();
    el2.caps.hypervisor_present = true;
    el2.vm_launch_ready = true;
    el2.prep_attempts = 1;
    el2.prep_success = 1;
    assert_eq!(
        smoke_profile(el2, "launch-prepared", "tier1"),
        ("el2", true, 1000, "staged", true)
    );
}

#[test_case]
fn operational_helpers_cover_trace_and_passthrough_logic() {
    assert!(can_trace_from_flags(true, true));
    assert!(!can_trace_from_flags(true, false));
    assert!(can_passthrough_from_flags(true, true));
    assert!(!can_passthrough_from_flags(true, false));
    assert!(can_resume_from_flags(true, true));
    assert!(!can_resume_from_flags(true, false));
    assert!(can_enable_nested_from_flags(true, true));
    assert!(!can_enable_nested_from_flags(true, false));
    assert!(can_virtualize_time_from_flags(true, true));
    assert!(!can_virtualize_time_from_flags(true, false));
    assert_eq!(observability_tier_from_flags(true, true), "full");
    assert_eq!(observability_tier_from_flags(true, false), "partial");
    assert_eq!(observability_tier_from_flags(false, false), "minimal");
    assert!(snapshot_ready_from_flags(true, true));
    assert!(!snapshot_ready_from_flags(true, false));
    assert!(dirty_logging_ready_from_flags(true, true));
    assert!(!dirty_logging_ready_from_flags(true, false));
    assert!(live_migration_ready_from_flags(true, true, true));
    assert!(!live_migration_ready_from_flags(true, true, false));
    assert_eq!(
        advanced_operations_tier(true, true, true),
        "hypervisor-grade"
    );
    assert_eq!(advanced_operations_tier(true, false, true), "advanced");
    assert_eq!(advanced_operations_tier(false, false, false), "baseline");
    assert_eq!(
        advanced_operations_profile(true, true, true),
        (true, true, true, "hypervisor-grade")
    );
    assert_eq!(primary_lifecycle("prepared", "uninitialized"), "prepared");
    assert_eq!(primary_lifecycle("uninitialized", "active"), "active");
    assert_eq!(
        guest_lifecycle_profile("active", true, true, "advanced"),
        ("active", true, true, "advanced")
    );
    assert_eq!(
        guest_control_profile(true, true, true, true),
        ("guest-control-ready", true, true, true)
    );
    assert_eq!(
        guest_control_profile(true, false, false, true),
        ("guest-control-partial", true, false, false)
    );
    assert_eq!(
        guest_control_profile(false, false, true, false),
        ("guest-control-prepared", false, false, true)
    );
    assert_eq!(
        operational_smoke_profile("ready", true, true, true, true, true),
        ("ready", true, true, true)
    );
    assert_eq!(
        operational_smoke_profile("staged", false, false, false, false, false),
        ("staged", false, false, false)
    );
}

#[test_case]
fn summarize_operations_derives_common_advanced_flags() {
    let summary = summarize_operations(VirtOperationFlags {
        control_plane_ready: true,
        exit_tracing_ready: true,
        interrupt_virtualization_ready: true,
        time_virtualization_ready: true,
        monitoring_ready: true,
        resume_ready: true,
        guest_entry_ready: true,
        state_save_restore_ready: true,
        trap_handling_ready: true,
        memory_isolation_ready: true,
        device_passthrough_ready: true,
    });
    assert_eq!(summary.observability_tier, "full");
    assert!(summary.snapshot_ready);
    assert!(summary.dirty_logging_ready);
    assert!(summary.live_migration_ready);
    assert_eq!(summary.advanced_operations_tier, "hypervisor-grade");
}

#[test_case]
fn virtualization_runtime_governor_matches_latency_and_background_profiles() {
    let latency = virtualization_runtime_governor(
        "LatencyCritical",
        "latency-critical",
        "backend-full",
        "low-latency",
    );
    assert_eq!(latency.governor_class, "latency-focused");
    assert_eq!(latency.latency_bias, "aggressive");
    assert_eq!(latency.energy_bias, "performance");
    assert_eq!(
        latency.scheduler,
        VirtualizationSchedulerTuning {
            threshold_divisor: 2,
            threshold_multiplier: 1,
            burst_divisor: 2,
            burst_multiplier: 1,
        }
    );
    assert_eq!(
        latency.rebalance,
        VirtualizationRebalanceTuning {
            threshold_divisor: 2,
            batch_multiplier: 2,
            prefer_local_skip_budget_divisor: 2,
        }
    );
    assert_eq!(
        latency.power,
        VirtualizationPowerTuning {
            prefer_active_pstate: true,
            prefer_shallow_idle: true,
        }
    );

    let background = virtualization_runtime_governor(
        "Background",
        "background",
        "backend-blocked",
        "throughput",
    );
    assert_eq!(background.governor_class, "background-optimized");
    assert_eq!(background.latency_bias, "relaxed");
    assert_eq!(background.energy_bias, "saving");

    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_governor_policy_profile(Some(
        crate::config::VirtualizationGovernorProfile {
            governor_class: crate::config::VirtualizationGovernorClass::Performance,
        },
    ));
    let forced =
        virtualization_runtime_governor("Balanced", "background", "backend-blocked", "throughput");
    assert_eq!(forced.governor_class, "performance-governor");
    assert_eq!(forced.latency_bias, "aggressive");
    assert_eq!(forced.energy_bias, "performance");
    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn operational_detail_helpers_classify_magic_values_consistently() {
    assert!(!control_is_operational(CONTROL_NONE));
    assert!(!control_is_operational(CONTROL_DETECTED));
    assert!(control_is_operational(CONTROL_EL2_ACTIVE));
    assert!(!trap_is_operational(TRAP_NOT_READY));
    assert!(trap_is_operational(TRAP_VMX_READY));
    assert!(!interrupt_is_operational(INTERRUPT_BASIC));
    assert!(interrupt_is_operational(INTERRUPT_VMX_READY));
    assert!(!time_is_operational(TIME_BASIC));
    assert!(time_is_operational(TIME_VMX_READY));
    assert!(backend_has_full_runtime(BACKEND_VMX_ACTIVE));
    assert!(backend_has_full_runtime(BACKEND_EL2_FULL));
    assert!(!backend_has_full_runtime(BACKEND_VMX_ENABLED));
    assert!(capability_has_entry_path(CAPABILITY_VMX_ACTIVE));
    assert!(capability_has_entry_path(CAPABILITY_EL2_ENABLED));
    assert!(!capability_has_entry_path(CAPABILITY_VMX_DETECTED));
}

#[test_case]
fn guest_runtime_profile_combines_control_and_resume_state() {
    assert_eq!(
        guest_runtime_profile(GuestRuntimeFlags {
            launch_ready: true,
            control_ready: true,
            trap_ready: true,
            resume_ready: true,
            snapshot_ready: true,
        }),
        ("guest-control-ready", true, true, true, true)
    );
    assert_eq!(
        guest_runtime_profile(GuestRuntimeFlags {
            launch_ready: true,
            control_ready: true,
            trap_ready: false,
            resume_ready: false,
            snapshot_ready: false,
        }),
        ("guest-control-partial", true, false, false, false)
    );
}

#[test_case]
fn guest_exit_profile_combines_trap_trace_and_timer_state() {
    assert_eq!(
        guest_exit_profile(GuestExitFlags {
            launch_ready: true,
            trap_ready: true,
            trace_ready: true,
            interrupt_ready: true,
            time_ready: true,
        }),
        ("guest-exit-ready", true, true, true, true)
    );
    assert_eq!(
        guest_exit_profile(GuestExitFlags {
            launch_ready: true,
            trap_ready: false,
            trace_ready: true,
            interrupt_ready: false,
            time_ready: false,
        }),
        ("guest-exit-partial", false, true, false, false)
    );
}

#[test_case]
fn guest_launch_profile_combines_entry_control_and_isolation() {
    assert_eq!(
        guest_launch_profile(GuestLaunchFlags {
            launch_ready: true,
            control_ready: true,
            guest_entry_ready: true,
            memory_isolation_ready: true,
        }),
        ("guest-launch-ready", true, true, true)
    );
    assert_eq!(
        guest_launch_profile(GuestLaunchFlags {
            launch_ready: true,
            control_ready: false,
            guest_entry_ready: true,
            memory_isolation_ready: false,
        }),
        ("guest-launch-partial", false, true, false)
    );
}

#[test_case]
fn lifecycle_transition_helpers_cover_activate_reset_and_teardown() {
    assert_eq!(
        activate_lifecycle_code(true, true),
        Some(LIFECYCLE_CODE_ACTIVE)
    );
    assert_eq!(activate_lifecycle_code(true, false), None);
    assert_eq!(reset_lifecycle_code(true), Some(LIFECYCLE_CODE_PREPARED));
    assert_eq!(reset_lifecycle_code(false), None);
    assert_eq!(
        teardown_lifecycle_code(LIFECYCLE_CODE_ACTIVE),
        Some(LIFECYCLE_CODE_TORN_DOWN)
    );
    assert_eq!(teardown_lifecycle_code(LIFECYCLE_CODE_UNINITIALIZED), None);
    assert!(has_launch_context(LIFECYCLE_STATE_ACTIVE));
    assert!(has_launch_context(LIFECYCLE_STATE_PREPARED));
    assert!(!has_launch_context(LIFECYCLE_STATE_FAILED));
}

#[test_case]
fn guest_operation_profile_aggregates_launch_runtime_and_exit() {
    let profile = guest_operation_profile(
        GuestLaunchFlags {
            launch_ready: true,
            control_ready: true,
            guest_entry_ready: true,
            memory_isolation_ready: true,
        },
        GuestRuntimeFlags {
            launch_ready: true,
            control_ready: true,
            trap_ready: true,
            resume_ready: true,
            snapshot_ready: true,
        },
        GuestExitFlags {
            launch_ready: true,
            trap_ready: true,
            trace_ready: true,
            interrupt_ready: true,
            time_ready: true,
        },
    );
    assert_eq!(profile.launch_stage, "guest-launch-ready");
    assert_eq!(profile.runtime_stage, "guest-control-ready");
    assert_eq!(profile.exit_stage, "guest-exit-ready");
    assert!(profile.control_ready);
    assert!(profile.trap_ready);
    assert!(profile.guest_entry_ready);
    assert!(profile.resume_ready);
    assert!(profile.snapshot_ready);
    assert!(profile.memory_isolation_ready);
}
