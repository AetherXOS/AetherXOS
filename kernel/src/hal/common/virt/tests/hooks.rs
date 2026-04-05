#![cfg(target_os = "none")]

use super::*;

#[test_case]
fn guest_operation_hooks_translate_profiles_into_operational_intent() {
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
    assert_eq!(
        guest_launch_hook(ready),
        GuestLaunchHook {
            action: "guest-launch-ready",
            allowed: true,
            requires_control_plane: false,
            requires_memory_isolation: false,
        }
    );
    assert_eq!(
        guest_runtime_hook(ready),
        GuestRuntimeHook {
            action: "guest-control-ready",
            allowed: true,
            resumable: true,
            snapshot_capable: true,
        }
    );
    assert_eq!(
        guest_exit_hook(ready),
        GuestExitHook {
            action: "guest-exit-ready",
            allowed: true,
            requires_trap_handling: false,
            requires_time_virtualization: false,
        }
    );

    let partial = guest_operation_profile(
        GuestLaunchFlags {
            launch_ready: true,
            control_ready: false,
            guest_entry_ready: true,
            memory_isolation_ready: false,
        },
        GuestRuntimeFlags {
            launch_ready: true,
            control_ready: true,
            trap_ready: false,
            resume_ready: false,
            snapshot_ready: false,
        },
        GuestExitFlags {
            launch_ready: true,
            trap_ready: false,
            trace_ready: true,
            interrupt_ready: false,
            time_ready: false,
        },
    );
    assert_eq!(guest_launch_hook(partial).action, "guest-launch-partial");
    assert!(!guest_launch_hook(partial).allowed);
    assert!(guest_launch_hook(partial).requires_control_plane);
    assert!(guest_launch_hook(partial).requires_memory_isolation);
    assert_eq!(guest_runtime_hook(partial).action, "guest-control-partial");
    assert!(!guest_runtime_hook(partial).allowed);
    assert_eq!(guest_exit_hook(partial).action, "guest-exit-partial");
    assert!(!guest_exit_hook(partial).allowed);
    assert!(guest_exit_hook(partial).requires_trap_handling);
    assert!(guest_exit_hook(partial).requires_time_virtualization);
    assert_eq!(
        guest_operation_hooks(ready),
        GuestOperationHooks {
            launch: guest_launch_hook(ready),
            runtime: guest_runtime_hook(ready),
            exit: guest_exit_hook(ready),
        }
    );
    assert_eq!(
        guest_operation_plan(guest_operation_hooks(ready)),
        GuestOperationPlan {
            next_action: "guest-launch-ready",
            launch_allowed: true,
            runtime_allowed: true,
            exit_allowed: true,
            resume_allowed: true,
        }
    );
    assert_eq!(
        guest_resume_intent(guest_operation_hooks(ready)),
        GuestResumeIntent {
            action: "guest-control-ready",
            allowed: true,
            requires_snapshot_capable_state: false,
        }
    );
    assert_eq!(
        guest_entry_intent(guest_operation_hooks(ready)),
        GuestEntryIntent {
            action: "guest-launch-ready",
            allowed: true,
            requires_memory_isolation: false,
        }
    );
    assert_eq!(
        guest_trap_intent(guest_operation_hooks(ready)),
        GuestTrapIntent {
            action: "guest-exit-ready",
            allowed: true,
            requires_time_virtualization: false,
        }
    );
    assert_eq!(
        guest_entry_operation(guest_entry_intent(guest_operation_hooks(ready))),
        crate::hal::common::virt::GuestRuntimeOperation {
            phase: "entry",
            action: "guest-launch-ready",
            allowed: true,
        }
    );
    assert_eq!(
        guest_resume_operation(guest_resume_intent(guest_operation_hooks(ready))),
        crate::hal::common::virt::GuestRuntimeOperation {
            phase: "resume",
            action: "guest-control-ready",
            allowed: true,
        }
    );
    assert_eq!(
        guest_trap_operation(guest_trap_intent(guest_operation_hooks(ready))),
        crate::hal::common::virt::GuestRuntimeOperation {
            phase: "trap",
            action: "guest-exit-ready",
            allowed: true,
        }
    );
    assert_eq!(
        guest_operation_decision(guest_operation_hooks(ready)),
        GuestOperationDecision {
            next_action: "guest-launch-ready",
            launch_action: "guest-launch-ready",
            runtime_action: "guest-control-ready",
            exit_action: "guest-exit-ready",
            resume_allowed: true,
            entry_allowed: true,
            trap_allowed: true,
        }
    );
    assert_eq!(
        guest_operation_bundle(ready),
        GuestOperationBundle {
            hooks: guest_operation_hooks(ready),
            plan: guest_operation_plan(guest_operation_hooks(ready)),
            decision: guest_operation_decision(guest_operation_hooks(ready)),
            resume_intent: guest_resume_intent(guest_operation_hooks(ready)),
            entry_intent: guest_entry_intent(guest_operation_hooks(ready)),
            trap_intent: guest_trap_intent(guest_operation_hooks(ready)),
            entry_operation: guest_entry_operation(guest_entry_intent(guest_operation_hooks(
                ready
            ))),
            resume_operation: guest_resume_operation(guest_resume_intent(guest_operation_hooks(
                ready
            ))),
            trap_operation: guest_trap_operation(guest_trap_intent(guest_operation_hooks(ready))),
        }
    );
    assert_eq!(
        execute_guest_entry(guest_operation_bundle(ready)),
        GuestExecutionStep {
            phase: "entry",
            action: "guest-launch-ready",
            allowed: true,
            blocked_by: None,
        }
    );
    assert_eq!(
        execute_guest_resume(guest_operation_bundle(ready)),
        GuestExecutionStep {
            phase: "resume",
            action: "guest-control-ready",
            allowed: true,
            blocked_by: None,
        }
    );
    assert_eq!(
        execute_guest_trap(guest_operation_bundle(ready)),
        GuestExecutionStep {
            phase: "trap",
            action: "guest-exit-ready",
            allowed: true,
            blocked_by: None,
        }
    );
    assert_eq!(
        guest_runtime_execution(guest_operation_bundle(ready)),
        GuestRuntimeExecution {
            next_action: "guest-launch-ready",
            entry: execute_guest_entry(guest_operation_bundle(ready)),
            resume: execute_guest_resume(guest_operation_bundle(ready)),
            trap: execute_guest_trap(guest_operation_bundle(ready)),
        }
    );
    assert_eq!(
        guest_orchestration_summary(guest_operation_bundle(ready)),
        GuestTransitionState {
            stage: "transition-ready",
            selected_phase: "resume",
            selected_action: "guest-control-ready",
            ready: true,
            blocked_by: None,
        }
    );
    assert_eq!(backend_family("vmx:vmxon+vmcs"), "vmx");
    assert_eq!(backend_family("svm:enabled+vmcb"), "svm");
    assert_eq!(backend_family("el2:active+gic+timer"), "el2");
    assert_eq!(
        backend_operational_path(
            "vmx:vmxon+vmcs",
            guest_orchestration_summary(guest_operation_bundle(ready))
        ),
        "vmx-resume"
    );
    assert_eq!(
        guest_backend_execution(
            "vmx:vmxon+vmcs",
            "vmx:entry+vmcs+assist",
            "vmx:ept-like+exit-controls",
            guest_orchestration_summary(guest_operation_bundle(ready))
        ),
        GuestBackendExecution {
            backend_family: "vmx",
            backend_detail: "vmx:vmxon+vmcs",
            capability_detail: "vmx:entry+vmcs+assist",
            feature_detail: "vmx:ept-like+exit-controls",
            transition_stage: "transition-ready",
            selected_phase: "resume",
            selected_action: "guest-control-ready",
            operational_path: "vmx-resume",
            ready: true,
            blocked_by: None,
            policy: crate::hal::common::virt::VirtualizationExecutionPolicy {
                runtime: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
                cargo: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
                effective: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
            },
        }
    );
    assert_eq!(
        backend_runtime_step(guest_backend_execution(
            "vmx:vmxon+vmcs",
            "vmx:entry+vmcs+assist",
            "vmx:ept-like+exit-controls",
            guest_orchestration_summary(guest_operation_bundle(ready))
        )),
        "resume-vmx-vcpu"
    );
    assert_eq!(
        guest_backend_runtime_plan(guest_backend_execution(
            "vmx:vmxon+vmcs",
            "vmx:entry+vmcs+assist",
            "vmx:ept-like+exit-controls",
            guest_orchestration_summary(guest_operation_bundle(ready))
        )),
        GuestBackendRuntimePlan {
            backend_family: "vmx",
            operational_path: "vmx-resume",
            transition_stage: "transition-ready",
            step: "resume-vmx-vcpu",
            aux_step: "prepare-live-migration-state",
            operation_class: "full",
            selected_mode: "backend-full",
            runtime_strategy: "stateful-balanced",
            runtime_budget_class: "wide",
            ready: true,
            blocked_by: None,
            policy_limited_by: None,
            policy: crate::hal::common::virt::VirtualizationExecutionPolicy {
                runtime: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
                cargo: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
                effective: crate::config::VirtualizationRuntimeProfile {
                    telemetry: true,
                    platform_lifecycle: true,
                    entry: true,
                    resume: true,
                    trap_dispatch: true,
                    nested: true,
                    time_virtualization: true,
                    device_passthrough: true,
                    snapshot: true,
                    dirty_logging: true,
                    live_migration: true,
                    trap_tracing: true,
                },
            },
        }
    );
    let ready_dispatch =
        guest_runtime_dispatch_hint(guest_backend_runtime_plan(guest_backend_execution(
            "vmx:vmxon+vmcs",
            "vmx:entry+vmcs+assist",
            "vmx:ept-like+exit-controls",
            guest_orchestration_summary(guest_operation_bundle(ready)),
        )));
    assert_eq!(
        ready_dispatch,
        GuestRuntimeDispatchHint {
            runtime_strategy: "stateful-balanced",
            runtime_budget_class: "wide",
            dispatch_class: "balanced",
            preemption_policy: "preemptible",
        }
    );
    assert_eq!(
        guest_runtime_scheduling_profile(ready_dispatch),
        crate::hal::common::virt::GuestRuntimeSchedulingProfile {
            scheduler_lane: "balanced",
            dispatch_window: "adaptive-window",
            dispatch_class: "balanced",
            preemption_policy: "preemptible",
        }
    );
    assert_eq!(
        guest_backend_state_machine(guest_backend_execution(
            "vmx:vmxon+vmcs",
            "vmx:entry+vmcs+assist",
            "vmx:ept-like+exit-controls",
            guest_orchestration_summary(guest_operation_bundle(ready))
        )),
        GuestBackendStateMachine {
            backend_family: "vmx",
            detect_state: "detected",
            prepare_state: "prepared",
            capability_state: "active",
            feature_state: "active",
            launch_state: "prepared",
            resume_state: "ready",
            trap_state: "prepared",
            policy_limited_by: None,
        }
    );
    assert_eq!(
        guest_next_runtime_action(guest_operation_hooks(ready)),
        "guest-control-ready"
    );
}
