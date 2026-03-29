use super::ops;

pub fn guest_operation_bundle() -> crate::hal::common::virt::GuestOperationBundle {
    crate::hal::common::virt::guest_operation_bundle(ops::guest_operation_profile())
}

pub fn guest_runtime_execution() -> crate::hal::common::virt::GuestRuntimeExecution {
    crate::hal::common::virt::guest_runtime_execution(guest_operation_bundle())
}

pub fn guest_transition_state() -> crate::hal::common::virt::GuestTransitionState {
    crate::hal::common::virt::guest_orchestration_summary(guest_operation_bundle())
}

pub fn guest_backend_execution() -> crate::hal::common::virt::GuestBackendExecution {
    let detail = crate::hal::aarch64::virt::detail::summarize(
        crate::hal::aarch64::virt::status(),
        true,
        3,
        1_000_000,
        true,
    );
    crate::hal::common::virt::guest_backend_execution(
        detail.backend_detail,
        detail.capability_detail,
        detail.feature_detail,
        guest_transition_state(),
    )
}

pub fn guest_backend_runtime_plan() -> crate::hal::common::virt::GuestBackendRuntimePlan {
    crate::hal::common::virt::guest_backend_runtime_plan(guest_backend_execution())
}

pub fn guest_runtime_dispatch_hint() -> crate::hal::common::virt::GuestRuntimeDispatchHint {
    crate::hal::common::virt::guest_runtime_dispatch_hint(guest_backend_runtime_plan())
}

pub fn guest_runtime_scheduling_profile() -> crate::hal::common::virt::GuestRuntimeSchedulingProfile
{
    crate::hal::common::virt::guest_runtime_scheduling_profile(guest_runtime_dispatch_hint())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn guest_runtime_operation_class() -> &'static str {
    guest_backend_runtime_plan().operation_class
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn guest_runtime_blocked_by() -> Option<&'static str> {
    guest_backend_runtime_plan().blocked_by
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn guest_runtime_policy_limited_by() -> Option<&'static str> {
    guest_backend_runtime_plan().policy_limited_by
}

pub fn guest_backend_state_machine() -> crate::hal::common::virt::GuestBackendStateMachine {
    crate::hal::common::virt::guest_backend_state_machine(guest_backend_execution())
}

pub fn guest_launch_hook() -> crate::hal::common::virt::GuestLaunchHook {
    guest_operation_bundle().hooks.launch
}

pub fn guest_runtime_hook() -> crate::hal::common::virt::GuestRuntimeHook {
    guest_operation_bundle().hooks.runtime
}

pub fn guest_exit_hook() -> crate::hal::common::virt::GuestExitHook {
    guest_operation_bundle().hooks.exit
}

pub fn guest_operation_hooks() -> crate::hal::common::virt::GuestOperationHooks {
    guest_operation_bundle().hooks
}

pub fn guest_operation_plan() -> crate::hal::common::virt::GuestOperationPlan {
    guest_operation_bundle().plan
}

pub fn guest_operation_decision() -> crate::hal::common::virt::GuestOperationDecision {
    guest_operation_bundle().decision
}

pub fn guest_next_runtime_action() -> &'static str {
    guest_operation_bundle().decision.next_action
}

pub fn guest_resume_intent() -> crate::hal::common::virt::GuestResumeIntent {
    guest_operation_bundle().resume_intent
}

pub fn guest_entry_intent() -> crate::hal::common::virt::GuestEntryIntent {
    guest_operation_bundle().entry_intent
}

pub fn guest_trap_intent() -> crate::hal::common::virt::GuestTrapIntent {
    guest_operation_bundle().trap_intent
}

pub fn guest_entry_operation() -> crate::hal::common::virt::GuestRuntimeOperation {
    guest_operation_bundle().entry_operation
}

pub fn guest_resume_operation() -> crate::hal::common::virt::GuestRuntimeOperation {
    guest_operation_bundle().resume_operation
}

pub fn guest_trap_operation() -> crate::hal::common::virt::GuestRuntimeOperation {
    guest_operation_bundle().trap_operation
}
