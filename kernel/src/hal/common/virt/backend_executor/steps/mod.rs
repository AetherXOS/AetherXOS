mod el2_runtime;
mod svm_runtime;
mod vmx_runtime;

use super::types::*;
use crate::hal::common::virt::{
    feature_backend_mode, runtime_budget_class_for_mode, runtime_dispatch_class,
    runtime_dispatch_window, runtime_operation_class, runtime_preemption_policy,
    runtime_scheduler_lane, runtime_strategy_for_mode, BACKEND_MODE_BASIC, BACKEND_MODE_BLOCKED,
    BLOCKED_BY_DEVICE_PASSTHROUGH_POLICY, BLOCKED_BY_DIRTY_LOGGING_POLICY,
    BLOCKED_BY_LIVE_MIGRATION_POLICY, BLOCKED_BY_NESTED_POLICY,
    BLOCKED_BY_TIME_VIRTUALIZATION_POLICY, BLOCKED_BY_TRAP_TRACING_POLICY,
    RUNTIME_AUX_STEP_CHECKPOINT_SNAPSHOT, RUNTIME_AUX_STEP_ENABLE_DIRTY_LOGGING,
    RUNTIME_AUX_STEP_NONE, RUNTIME_AUX_STEP_PREPARE_LIVE_MIGRATION,
    RUNTIME_AUX_STEP_RECORD_TRAP_TRACE, RUNTIME_PATH_BLOCKED, RUNTIME_STEP_HOLD_BLOCKED_STATE,
    RUNTIME_STEP_RUN_GENERIC_TRANSITION,
};

#[inline(always)]
pub fn backend_runtime_step(execution: GuestBackendExecution) -> &'static str {
    vmx_runtime::runtime_step(execution)
        .or_else(|| svm_runtime::runtime_step(execution))
        .or_else(|| el2_runtime::runtime_step(execution))
        .unwrap_or_else(|| {
            if execution.operational_path == RUNTIME_PATH_BLOCKED {
                RUNTIME_STEP_HOLD_BLOCKED_STATE
            } else {
                RUNTIME_STEP_RUN_GENERIC_TRANSITION
            }
        })
}

#[inline(always)]
pub fn backend_runtime_aux_step(
    execution: GuestBackendExecution,
    selected_mode: &'static str,
) -> &'static str {
    if selected_mode == BACKEND_MODE_BLOCKED {
        RUNTIME_AUX_STEP_NONE
    } else if execution.selected_phase == "trap"
        && selected_mode != BACKEND_MODE_BASIC
        && execution.policy.effective.trap_tracing
    {
        RUNTIME_AUX_STEP_RECORD_TRAP_TRACE
    } else if execution.selected_phase == "resume"
        && selected_mode != BACKEND_MODE_BASIC
        && execution.policy.effective.live_migration
    {
        RUNTIME_AUX_STEP_PREPARE_LIVE_MIGRATION
    } else if execution.selected_phase == "resume" && execution.policy.effective.snapshot {
        RUNTIME_AUX_STEP_CHECKPOINT_SNAPSHOT
    } else if matches!(execution.selected_phase, "entry" | "resume")
        && execution.policy.effective.dirty_logging
    {
        RUNTIME_AUX_STEP_ENABLE_DIRTY_LOGGING
    } else {
        RUNTIME_AUX_STEP_NONE
    }
}

#[inline(always)]
fn policy_limited_by(execution: GuestBackendExecution, step: &'static str) -> Option<&'static str> {
    if !step.ends_with("-basic") {
        return None;
    }

    match execution.selected_phase {
        "entry" => {
            if !execution.policy.effective.device_passthrough {
                Some(BLOCKED_BY_DEVICE_PASSTHROUGH_POLICY)
            } else if !execution.policy.effective.dirty_logging {
                Some(BLOCKED_BY_DIRTY_LOGGING_POLICY)
            } else {
                None
            }
        }
        "resume" => {
            if !execution.policy.effective.nested {
                Some(BLOCKED_BY_NESTED_POLICY)
            } else if !execution.policy.effective.live_migration {
                Some(BLOCKED_BY_LIVE_MIGRATION_POLICY)
            } else {
                None
            }
        }
        "trap" => {
            if !execution.policy.effective.time_virtualization {
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
pub(super) fn full_entry_enabled(execution: GuestBackendExecution) -> bool {
    execution.policy.effective.entry
        && execution.policy.effective.dirty_logging
        && execution.policy.effective.device_passthrough
}

#[inline(always)]
pub(super) fn full_resume_enabled(execution: GuestBackendExecution) -> bool {
    execution.policy.effective.resume
        && execution.policy.effective.live_migration
        && execution.policy.effective.nested
}

#[inline(always)]
pub(super) fn full_trap_enabled(execution: GuestBackendExecution) -> bool {
    execution.policy.effective.trap_dispatch
        && execution.policy.effective.trap_tracing
        && execution.policy.effective.time_virtualization
}

#[inline(always)]
fn selected_runtime_mode(
    execution: GuestBackendExecution,
    step: &'static str,
    blocked_by: Option<&'static str>,
) -> &'static str {
    let feature_ready = blocked_by.is_none() && !matches!(step, RUNTIME_STEP_HOLD_BLOCKED_STATE);

    match execution.selected_phase {
        "entry" => feature_backend_mode(feature_ready, execution.entry_scope(), None),
        "resume" => feature_backend_mode(
            feature_ready,
            execution.resume_scope(),
            Some(execution.nested_scope()),
        ),
        "trap" => feature_backend_mode(
            feature_ready,
            execution.trap_dispatch_scope(),
            Some(execution.time_virtualization_scope()),
        ),
        _ => feature_backend_mode(false, "fully-disabled", None),
    }
}

#[inline(always)]
pub fn guest_backend_runtime_plan(execution: GuestBackendExecution) -> GuestBackendRuntimePlan {
    let step = backend_runtime_step(execution);
    let blocked_by = execution.blocked_by;
    let selected_mode = selected_runtime_mode(execution, step, blocked_by);
    let operation_class = runtime_operation_class(step, blocked_by);
    let policy_limited_by = policy_limited_by(execution, step);
    GuestBackendRuntimePlan {
        backend_family: execution.backend_family,
        operational_path: execution.operational_path,
        transition_stage: execution.transition_stage,
        step,
        aux_step: backend_runtime_aux_step(execution, selected_mode),
        operation_class,
        selected_mode,
        runtime_strategy: runtime_strategy_for_mode(
            execution.selected_phase,
            selected_mode,
            operation_class,
        ),
        runtime_budget_class: runtime_budget_class_for_mode(selected_mode, operation_class),
        ready: execution.ready,
        blocked_by,
        policy_limited_by,
        policy: execution.policy,
    }
}

#[inline(always)]
pub fn guest_runtime_dispatch_hint(plan: GuestBackendRuntimePlan) -> GuestRuntimeDispatchHint {
    GuestRuntimeDispatchHint {
        runtime_strategy: plan.runtime_strategy,
        runtime_budget_class: plan.runtime_budget_class,
        dispatch_class: runtime_dispatch_class(plan.runtime_strategy, plan.runtime_budget_class),
        preemption_policy: runtime_preemption_policy(plan.selected_mode, plan.runtime_budget_class),
    }
}

#[inline(always)]
pub fn guest_runtime_scheduling_profile(
    hint: GuestRuntimeDispatchHint,
) -> GuestRuntimeSchedulingProfile {
    let execution_profile =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let (scheduler_lane, dispatch_window) = match execution_profile.scheduling_class {
        crate::config::VirtualizationExecutionClass::LatencyCritical => (
            crate::hal::common::virt::RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
            crate::hal::common::virt::RUNTIME_DISPATCH_WINDOW_SHORT,
        ),
        crate::config::VirtualizationExecutionClass::Background => (
            crate::hal::common::virt::RUNTIME_SCHED_LANE_BACKGROUND,
            crate::hal::common::virt::RUNTIME_DISPATCH_WINDOW_HOLD,
        ),
        crate::config::VirtualizationExecutionClass::Balanced => (
            runtime_scheduler_lane(hint.dispatch_class),
            runtime_dispatch_window(hint.dispatch_class, hint.preemption_policy),
        ),
    };
    GuestRuntimeSchedulingProfile {
        scheduler_lane,
        dispatch_window,
        dispatch_class: hint.dispatch_class,
        preemption_policy: hint.preemption_policy,
    }
}
