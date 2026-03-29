use super::types::*;
use crate::hal::common::virt::{
    BLOCKED_BY_DEVICE_PASSTHROUGH_POLICY, BLOCKED_BY_DIRTY_LOGGING_POLICY, BLOCKED_BY_ENTRY_POLICY,
    BLOCKED_BY_LIVE_MIGRATION_POLICY, BLOCKED_BY_NESTED_POLICY, BLOCKED_BY_RESUME_POLICY,
    BLOCKED_BY_TIME_VIRTUALIZATION_POLICY, BLOCKED_BY_TRAP_DISPATCH_POLICY,
    BLOCKED_BY_TRAP_TRACING_POLICY, READINESS_POLICY_LIMITED, TRANSITION_POLICY_LIMITED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBackendStateMachine {
    pub backend_family: &'static str,
    pub detect_state: &'static str,
    pub prepare_state: &'static str,
    pub capability_state: &'static str,
    pub feature_state: &'static str,
    pub launch_state: &'static str,
    pub resume_state: &'static str,
    pub trap_state: &'static str,
    pub policy_limited_by: Option<&'static str>,
}

#[inline(always)]
fn capability_state(execution: GuestBackendExecution) -> &'static str {
    if execution.capability_detail.ends_with("assist")
        || execution.capability_detail.ends_with("vmcb")
        || execution.capability_detail.ends_with("entry")
    {
        if execution.policy.effective.snapshot
            && execution.policy.effective.live_migration
            && execution.policy.effective.device_passthrough
        {
            "active"
        } else {
            READINESS_POLICY_LIMITED
        }
    } else if execution.capability_detail.contains("msr")
        || execution.capability_detail.contains("efer")
        || execution.capability_detail.contains("entry-controls")
    {
        "enabled"
    } else if execution.backend_family == "none" {
        "absent"
    } else {
        "detected"
    }
}

#[inline(always)]
fn feature_state(execution: GuestBackendExecution) -> &'static str {
    if execution.feature_detail.contains("exit-controls")
        || execution.feature_detail.contains("vmcb")
        || execution.feature_detail.contains("vtimer")
    {
        if execution.policy.effective.trap_tracing
            && execution.policy.effective.dirty_logging
            && execution.policy.effective.time_virtualization
        {
            "active"
        } else {
            READINESS_POLICY_LIMITED
        }
    } else if execution.feature_detail.contains("control") {
        "enabled"
    } else if execution.backend_family == "none" {
        "absent"
    } else {
        "detected"
    }
}

#[inline(always)]
fn state_for_phase(execution: GuestBackendExecution, phase: &'static str) -> &'static str {
    if execution.selected_phase == phase {
        if execution.ready {
            "ready"
        } else if execution.transition_stage == TRANSITION_POLICY_LIMITED {
            READINESS_POLICY_LIMITED
        } else {
            "blocked"
        }
    } else if execution.transition_stage == "transition-ready"
        || execution.transition_stage == "transition-partial"
        || execution.transition_stage == TRANSITION_POLICY_LIMITED
    {
        "prepared"
    } else {
        "idle"
    }
}

#[inline(always)]
fn policy_limited_by(execution: GuestBackendExecution) -> Option<&'static str> {
    match execution.selected_phase {
        "entry" => {
            if !execution.policy.effective.entry {
                Some(BLOCKED_BY_ENTRY_POLICY)
            } else if !execution.policy.effective.device_passthrough {
                Some(BLOCKED_BY_DEVICE_PASSTHROUGH_POLICY)
            } else if !execution.policy.effective.dirty_logging {
                Some(BLOCKED_BY_DIRTY_LOGGING_POLICY)
            } else {
                None
            }
        }
        "resume" => {
            if !execution.policy.effective.resume {
                Some(BLOCKED_BY_RESUME_POLICY)
            } else if !execution.policy.effective.nested {
                Some(BLOCKED_BY_NESTED_POLICY)
            } else if !execution.policy.effective.live_migration {
                Some(BLOCKED_BY_LIVE_MIGRATION_POLICY)
            } else {
                None
            }
        }
        "trap" => {
            if !execution.policy.effective.trap_dispatch {
                Some(BLOCKED_BY_TRAP_DISPATCH_POLICY)
            } else if !execution.policy.effective.time_virtualization {
                Some(BLOCKED_BY_TIME_VIRTUALIZATION_POLICY)
            } else if !execution.policy.effective.trap_tracing {
                Some(BLOCKED_BY_TRAP_TRACING_POLICY)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[inline(always)]
pub fn guest_backend_state_machine(execution: GuestBackendExecution) -> GuestBackendStateMachine {
    GuestBackendStateMachine {
        backend_family: execution.backend_family,
        detect_state: if execution.backend_family == "none" {
            "absent"
        } else {
            "detected"
        },
        prepare_state: if execution.transition_stage == "transition-blocked" {
            "blocked"
        } else {
            "prepared"
        },
        capability_state: capability_state(execution),
        feature_state: feature_state(execution),
        launch_state: state_for_phase(execution, "entry"),
        resume_state: state_for_phase(execution, "resume"),
        trap_state: state_for_phase(execution, "trap"),
        policy_limited_by: if execution.transition_stage == TRANSITION_POLICY_LIMITED {
            policy_limited_by(execution)
        } else {
            None
        },
    }
}
