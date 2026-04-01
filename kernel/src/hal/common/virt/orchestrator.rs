use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTransitionState {
    pub stage: &'static str,
    pub selected_phase: &'static str,
    pub selected_action: &'static str,
    pub ready: bool,
    pub blocked_by: Option<&'static str>,
}

#[inline(always)]
fn transition_stage(
    execution: GuestRuntimeExecution,
    selected: GuestExecutionStep,
    policy: crate::config::VirtualizationRuntimeProfile,
) -> &'static str {
    if selected.allowed {
        TRANSITION_READY
    } else if !policy.entry && selected.phase == "entry" {
        TRANSITION_POLICY_LIMITED
    } else if !policy.resume && selected.phase == "resume" {
        TRANSITION_POLICY_LIMITED
    } else if !policy.trap_dispatch && selected.phase == "trap" {
        TRANSITION_POLICY_LIMITED
    } else if !policy.trap_tracing && selected.phase == "trap" {
        TRANSITION_POLICY_LIMITED
    } else if !policy.snapshot && selected.phase == "resume" {
        TRANSITION_POLICY_LIMITED
    } else if execution.entry.allowed || execution.resume.allowed || execution.trap.allowed {
        TRANSITION_PARTIAL
    } else {
        TRANSITION_BLOCKED
    }
}

#[inline(always)]
pub fn guest_transition_state(execution: GuestRuntimeExecution) -> GuestTransitionState {
    let selected = if execution.resume.allowed {
        execution.resume
    } else if execution.entry.allowed {
        execution.entry
    } else {
        execution.trap
    };

    let stage = transition_stage(
        execution,
        selected,
        crate::config::KernelConfig::virtualization_effective_profile(),
    );

    GuestTransitionState {
        stage,
        selected_phase: selected.phase,
        selected_action: selected.action,
        ready: selected.allowed,
        blocked_by: selected.blocked_by,
    }
}

#[inline(always)]
pub fn guest_orchestration_summary(bundle: GuestOperationBundle) -> GuestTransitionState {
    guest_transition_state(guest_runtime_execution(bundle))
}
