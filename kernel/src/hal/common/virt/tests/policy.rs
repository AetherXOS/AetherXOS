use super::*;

#[test_case]
fn virtualization_policy_can_disable_snapshot_capability_in_hooks() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_snapshot_enabled(Some(false));

    let ready = guest_operation_profile(
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
    let hook = guest_runtime_hook(ready);
    let intent = guest_resume_intent(guest_operation_hooks(ready));

    assert!(!hook.snapshot_capable);
    assert!(intent.requires_snapshot_capable_state);

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_disable_trap_tracing_requirement() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(false));

    let partial = guest_operation_profile(
        GuestLaunchFlags {
            launch_ready: false,
            control_ready: false,
            guest_entry_ready: false,
            memory_isolation_ready: false,
        },
        GuestRuntimeFlags {
            launch_ready: false,
            control_ready: false,
            trap_ready: false,
            resume_ready: false,
            snapshot_ready: false,
        },
        GuestExitFlags {
            launch_ready: false,
            trap_ready: false,
            trace_ready: false,
            interrupt_ready: false,
            time_ready: false,
        },
    );
    let hook = guest_exit_hook(partial);
    let intent = guest_trap_intent(guest_operation_hooks(partial));

    assert!(!hook.allowed);
    assert!(!hook.requires_time_virtualization);
    assert!(!intent.requires_time_virtualization);

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_flows_into_backend_runtime_plan() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_live_migration_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_dirty_logging_enabled(Some(true));

    let ready = guest_operation_profile(
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

    let plan = guest_backend_runtime_plan(guest_backend_execution(
        "vmx:vmxon+vmcs",
        "vmx:entry+vmcs+assist",
        "vmx:ept-like+exit-controls",
        guest_orchestration_summary(guest_operation_bundle(ready)),
    ));

    assert_eq!(plan.step, "resume-vmx-vcpu-basic");
    assert_eq!(plan.aux_step, "checkpoint-snapshot-state");
    assert_eq!(plan.operation_class, "basic");
    assert_eq!(plan.selected_mode, "backend-basic");
    assert_eq!(plan.runtime_strategy, "generic-fallback");
    assert_eq!(plan.runtime_budget_class, "medium");
    assert_eq!(
        plan.policy_limited_by,
        Some("live-migration-policy-disabled")
    );
    let basic_dispatch = guest_runtime_dispatch_hint(plan);
    assert_eq!(
        basic_dispatch,
        GuestRuntimeDispatchHint {
            runtime_strategy: "generic-fallback",
            runtime_budget_class: "medium",
            dispatch_class: "balanced",
            preemption_policy: "cooperative",
        }
    );
    assert_eq!(
        guest_runtime_scheduling_profile(basic_dispatch),
        crate::hal::common::virt::GuestRuntimeSchedulingProfile {
            scheduler_lane: "balanced",
            dispatch_window: "adaptive-window",
            dispatch_class: "balanced",
            preemption_policy: "cooperative",
        }
    );
    assert!(!plan.policy.runtime.live_migration);
    assert!(plan.policy.cargo.live_migration);
    assert!(!plan.policy.effective.live_migration);
    assert!(plan.policy.effective.snapshot);

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_effective_policy_can_limit_transition_stage() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_snapshot_enabled(Some(false));

    let execution = GuestRuntimeExecution {
        next_action: "guest-control-ready",
        entry: GuestExecutionStep {
            phase: "entry",
            action: "guest-launch-ready",
            allowed: false,
            blocked_by: Some("control-plane"),
        },
        resume: GuestExecutionStep {
            phase: "resume",
            action: "guest-control-ready",
            allowed: false,
            blocked_by: Some("snapshot-policy-disabled"),
        },
        trap: GuestExecutionStep {
            phase: "trap",
            action: "guest-exit-ready",
            allowed: false,
            blocked_by: Some("trap-state"),
        },
    };

    let state = guest_transition_state(execution);
    assert_eq!(state.stage, "transition-policy-limited");
    assert_eq!(state.selected_phase, "trap");

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_downgrade_vmx_trap_runtime_step() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(false));

    let execution = guest_backend_execution(
        "vmx:vmxon+vmcs",
        "vmx:entry+vmcs+assist",
        "vmx:ept-like+exit-controls",
        GuestTransitionState {
            stage: "transition-policy-limited",
            selected_phase: "trap",
            selected_action: "guest-exit-ready",
            ready: false,
            blocked_by: Some("trap-tracing-policy-disabled"),
        },
    );

    let plan = guest_backend_runtime_plan(execution);
    assert_eq!(plan.step, "dispatch-vmx-trap-basic");
    assert_eq!(plan.aux_step, "no-aux-step");
    assert_eq!(plan.operation_class, "blocked");
    assert_eq!(plan.selected_mode, "backend-blocked");
    assert_eq!(plan.runtime_strategy, "conservative-hold");
    assert_eq!(plan.runtime_budget_class, "minimal");
    assert_eq!(plan.policy_limited_by, Some("trap-tracing-policy-disabled"));
    let blocked_dispatch = guest_runtime_dispatch_hint(plan);
    assert_eq!(
        blocked_dispatch,
        GuestRuntimeDispatchHint {
            runtime_strategy: "conservative-hold",
            runtime_budget_class: "minimal",
            dispatch_class: "conservative",
            preemption_policy: "hold",
        }
    );
    assert_eq!(
        guest_runtime_scheduling_profile(blocked_dispatch),
        crate::hal::common::virt::GuestRuntimeSchedulingProfile {
            scheduler_lane: "background",
            dispatch_window: "hold-window",
            dispatch_class: "conservative",
            preemption_policy: "hold",
        }
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_limit_backend_state_machine_with_nested_policy() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_nested_enabled(Some(false));

    let state = guest_backend_state_machine(guest_backend_execution(
        "svm:enabled+vmcb",
        "svm:efer+vmcb",
        "svm:npt-like+vmcb",
        GuestTransitionState {
            stage: "transition-policy-limited",
            selected_phase: "resume",
            selected_action: "guest-control-ready",
            ready: false,
            blocked_by: Some("nested-policy-disabled"),
        },
    ));

    assert_eq!(state.resume_state, "policy-limited");
    assert_eq!(state.policy_limited_by, Some("nested-policy-disabled"));

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_downgrade_el2_trap_when_time_virtualization_is_disabled() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_time_virtualization_enabled(Some(false));

    let execution = guest_backend_execution(
        "el2:active+gic+timer",
        "el2:timer+gic+entry",
        "el2:vgic+vtimer",
        GuestTransitionState {
            stage: "transition-policy-limited",
            selected_phase: "trap",
            selected_action: "guest-exit-ready",
            ready: false,
            blocked_by: Some("time-virtualization-policy-disabled"),
        },
    );

    let plan = guest_backend_runtime_plan(execution);
    assert_eq!(plan.step, "dispatch-el2-trap-basic");
    assert_eq!(plan.selected_mode, "backend-blocked");
    assert_eq!(plan.runtime_strategy, "conservative-hold");
    assert_eq!(plan.runtime_budget_class, "minimal");
    assert_eq!(
        plan.policy_limited_by,
        Some("time-virtualization-policy-disabled")
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_downgrade_svm_resume_when_nested_is_disabled() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_nested_enabled(Some(false));

    let execution = guest_backend_execution(
        "svm:enabled+vmcb",
        "svm:efer+vmcb",
        "svm:npt-like+vmcb",
        GuestTransitionState {
            stage: "transition-policy-limited",
            selected_phase: "resume",
            selected_action: "guest-control-ready",
            ready: false,
            blocked_by: Some("nested-policy-disabled"),
        },
    );

    let plan = guest_backend_runtime_plan(execution);
    assert_eq!(plan.step, "resume-svm-vcpu-basic");
    assert_eq!(plan.selected_mode, "backend-blocked");
    assert_eq!(plan.runtime_strategy, "conservative-hold");
    assert_eq!(plan.runtime_budget_class, "minimal");
    assert_eq!(plan.policy_limited_by, Some("nested-policy-disabled"));

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_can_downgrade_vmx_entry_when_passthrough_is_disabled() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_device_passthrough_enabled(Some(false));

    let execution = guest_backend_execution(
        "vmx:vmxon+vmcs",
        "vmx:entry+vmcs+assist",
        "vmx:ept-like+exit-controls",
        GuestTransitionState {
            stage: "transition-policy-limited",
            selected_phase: "entry",
            selected_action: "guest-launch-ready",
            ready: false,
            blocked_by: Some("device-passthrough-policy-disabled"),
        },
    );

    let plan = guest_backend_runtime_plan(execution);
    assert_eq!(plan.step, "prepare-vmcs-entry-basic");
    assert_eq!(plan.selected_mode, "backend-blocked");
    assert_eq!(plan.runtime_strategy, "conservative-hold");
    assert_eq!(plan.runtime_budget_class, "minimal");
    assert_eq!(
        plan.policy_limited_by,
        Some("device-passthrough-policy-disabled")
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}
