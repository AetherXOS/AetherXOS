pub const RUNTIME_PATH_BLOCKED: &str = "blocked-runtime";
pub const RUNTIME_PATH_GENERIC: &str = "generic-runtime";
pub const RUNTIME_PATH_VMX_ENTRY: &str = "vmx-entry";
pub const RUNTIME_PATH_VMX_RESUME: &str = "vmx-resume";
pub const RUNTIME_PATH_VMX_TRAP: &str = "vmx-trap-dispatch";
pub const RUNTIME_PATH_SVM_ENTRY: &str = "svm-entry";
pub const RUNTIME_PATH_SVM_RESUME: &str = "svm-resume";
pub const RUNTIME_PATH_SVM_TRAP: &str = "svm-trap-dispatch";
pub const RUNTIME_PATH_EL2_ENTRY: &str = "el2-entry";
pub const RUNTIME_PATH_EL2_RESUME: &str = "el2-resume";
pub const RUNTIME_PATH_EL2_TRAP: &str = "el2-trap-dispatch";

pub const RUNTIME_STEP_PREPARE_VMCS_ENTRY: &str = "prepare-vmcs-entry";
pub const RUNTIME_STEP_PREPARE_VMCS_ENTRY_BASIC: &str = "prepare-vmcs-entry-basic";
pub const RUNTIME_STEP_RESUME_VMX_VCPU: &str = "resume-vmx-vcpu";
pub const RUNTIME_STEP_RESUME_VMX_VCPU_BASIC: &str = "resume-vmx-vcpu-basic";
pub const RUNTIME_STEP_DISPATCH_VMX_TRAP: &str = "dispatch-vmx-trap";
pub const RUNTIME_STEP_DISPATCH_VMX_TRAP_BASIC: &str = "dispatch-vmx-trap-basic";
pub const RUNTIME_STEP_PREPARE_VMCB_ENTRY: &str = "prepare-vmcb-entry";
pub const RUNTIME_STEP_PREPARE_VMCB_ENTRY_BASIC: &str = "prepare-vmcb-entry-basic";
pub const RUNTIME_STEP_RESUME_SVM_VCPU: &str = "resume-svm-vcpu";
pub const RUNTIME_STEP_RESUME_SVM_VCPU_BASIC: &str = "resume-svm-vcpu-basic";
pub const RUNTIME_STEP_DISPATCH_SVM_TRAP: &str = "dispatch-svm-trap";
pub const RUNTIME_STEP_DISPATCH_SVM_TRAP_BASIC: &str = "dispatch-svm-trap-basic";
pub const RUNTIME_STEP_PREPARE_EL2_ENTRY: &str = "prepare-el2-entry";
pub const RUNTIME_STEP_PREPARE_EL2_ENTRY_BASIC: &str = "prepare-el2-entry-basic";
pub const RUNTIME_STEP_RESUME_EL2_GUEST: &str = "resume-el2-guest";
pub const RUNTIME_STEP_RESUME_EL2_GUEST_BASIC: &str = "resume-el2-guest-basic";
pub const RUNTIME_STEP_DISPATCH_EL2_TRAP: &str = "dispatch-el2-trap";
pub const RUNTIME_STEP_DISPATCH_EL2_TRAP_BASIC: &str = "dispatch-el2-trap-basic";
pub const RUNTIME_STEP_HOLD_BLOCKED_STATE: &str = "hold-blocked-state";
pub const RUNTIME_STEP_RUN_GENERIC_TRANSITION: &str = "run-generic-transition";

pub const RUNTIME_AUX_STEP_NONE: &str = "no-aux-step";
pub const RUNTIME_AUX_STEP_CHECKPOINT_SNAPSHOT: &str = "checkpoint-snapshot-state";
pub const RUNTIME_AUX_STEP_ENABLE_DIRTY_LOGGING: &str = "enable-dirty-log-tracking";
pub const RUNTIME_AUX_STEP_PREPARE_LIVE_MIGRATION: &str = "prepare-live-migration-state";
pub const RUNTIME_AUX_STEP_RECORD_TRAP_TRACE: &str = "record-trap-trace";

pub const OPERATION_CLASS_FULL: &str = "full";
pub const OPERATION_CLASS_BASIC: &str = "basic";
pub const OPERATION_CLASS_BLOCKED: &str = "blocked";

pub const RUNTIME_STRATEGY_EVENT_FASTPATH: &str = "event-fastpath";
pub const RUNTIME_STRATEGY_STATEFUL_BALANCED: &str = "stateful-balanced";
pub const RUNTIME_STRATEGY_CONSERVATIVE_HOLD: &str = "conservative-hold";
pub const RUNTIME_STRATEGY_GENERIC_FALLBACK: &str = "generic-fallback";

pub const RUNTIME_BUDGET_WIDE: &str = "wide";
pub const RUNTIME_BUDGET_MEDIUM: &str = "medium";
pub const RUNTIME_BUDGET_MINIMAL: &str = "minimal";

pub const RUNTIME_DISPATCH_LATENCY_SAFE: &str = "latency-safe";
pub const RUNTIME_DISPATCH_BALANCED: &str = "balanced";
pub const RUNTIME_DISPATCH_CONSERVATIVE: &str = "conservative";

pub const RUNTIME_PREEMPT_PREEMPTIBLE: &str = "preemptible";
pub const RUNTIME_PREEMPT_COOPERATIVE: &str = "cooperative";
pub const RUNTIME_PREEMPT_HOLD: &str = "hold";

pub const RUNTIME_SCHED_LANE_LATENCY_CRITICAL: &str = "latency-critical";
pub const RUNTIME_SCHED_LANE_BALANCED: &str = "balanced";
pub const RUNTIME_SCHED_LANE_BACKGROUND: &str = "background";

pub const RUNTIME_DISPATCH_WINDOW_SHORT: &str = "short-window";
pub const RUNTIME_DISPATCH_WINDOW_ADAPTIVE: &str = "adaptive-window";
pub const RUNTIME_DISPATCH_WINDOW_HOLD: &str = "hold-window";

pub const GOVERNOR_CLASS_PERFORMANCE: &str = "performance-governor";
pub const GOVERNOR_CLASS_BALANCED: &str = "balanced";
pub const GOVERNOR_CLASS_EFFICIENCY: &str = "efficiency-governor";
pub const GOVERNOR_CLASS_LATENCY_FOCUSED: &str = "latency-focused";
pub const GOVERNOR_CLASS_BACKGROUND_OPTIMIZED: &str = "background-optimized";
pub const GOVERNOR_BIAS_AGGRESSIVE: &str = "aggressive";
pub const GOVERNOR_BIAS_BALANCED: &str = "balanced";
pub const GOVERNOR_BIAS_RELAXED: &str = "relaxed";
pub const GOVERNOR_ENERGY_PERFORMANCE: &str = "performance";
pub const GOVERNOR_ENERGY_BALANCED: &str = "balanced";
pub const GOVERNOR_ENERGY_SAVING: &str = "saving";

pub const BLOCKED_BY_SNAPSHOT_POLICY: &str = "snapshot-policy-disabled";
pub const BLOCKED_BY_SNAPSHOT_COMPILETIME: &str = "snapshot-compiletime-disabled";
pub const BLOCKED_BY_ENTRY_POLICY: &str = "entry-policy-disabled";
pub const BLOCKED_BY_ENTRY_COMPILETIME: &str = "entry-compiletime-disabled";
pub const BLOCKED_BY_RESUME_POLICY: &str = "resume-policy-disabled";
pub const BLOCKED_BY_RESUME_COMPILETIME: &str = "resume-compiletime-disabled";
pub const BLOCKED_BY_TRAP_DISPATCH_POLICY: &str = "trap-dispatch-policy-disabled";
pub const BLOCKED_BY_TRAP_DISPATCH_COMPILETIME: &str = "trap-dispatch-compiletime-disabled";
pub const BLOCKED_BY_TRAP_TRACING_POLICY: &str = "trap-tracing-policy-disabled";
pub const BLOCKED_BY_TRAP_TRACING_COMPILETIME: &str = "trap-tracing-compiletime-disabled";
pub const BLOCKED_BY_NESTED_POLICY: &str = "nested-policy-disabled";
pub const BLOCKED_BY_TIME_VIRTUALIZATION_POLICY: &str = "time-virtualization-policy-disabled";
pub const BLOCKED_BY_DEVICE_PASSTHROUGH_POLICY: &str = "device-passthrough-policy-disabled";
pub const BLOCKED_BY_LIVE_MIGRATION_POLICY: &str = "live-migration-policy-disabled";
pub const BLOCKED_BY_DIRTY_LOGGING_POLICY: &str = "dirty-logging-policy-disabled";

pub const TRANSITION_READY: &str = "transition-ready";
pub const TRANSITION_PARTIAL: &str = "transition-partial";
pub const TRANSITION_BLOCKED: &str = "transition-blocked";
pub const TRANSITION_POLICY_LIMITED: &str = "transition-policy-limited";

#[inline(always)]
pub fn runtime_operation_class(
    step: &'static str,
    blocked_by: Option<&'static str>,
) -> &'static str {
    if blocked_by.is_some() || step == RUNTIME_STEP_HOLD_BLOCKED_STATE {
        OPERATION_CLASS_BLOCKED
    } else if step.ends_with("-basic") {
        OPERATION_CLASS_BASIC
    } else {
        OPERATION_CLASS_FULL
    }
}

#[inline(always)]
pub fn runtime_strategy_for_mode(
    selected_phase: &'static str,
    selected_mode: &'static str,
    operation_class: &'static str,
) -> &'static str {
    if operation_class == OPERATION_CLASS_BLOCKED
        || selected_mode == super::backend::BACKEND_MODE_BLOCKED
    {
        RUNTIME_STRATEGY_CONSERVATIVE_HOLD
    } else if selected_phase == "trap" && selected_mode == super::backend::BACKEND_MODE_FULL {
        RUNTIME_STRATEGY_EVENT_FASTPATH
    } else if selected_mode == super::backend::BACKEND_MODE_FULL {
        RUNTIME_STRATEGY_STATEFUL_BALANCED
    } else {
        RUNTIME_STRATEGY_GENERIC_FALLBACK
    }
}

#[inline(always)]
pub fn runtime_budget_class_for_mode(
    selected_mode: &'static str,
    operation_class: &'static str,
) -> &'static str {
    if operation_class == OPERATION_CLASS_BLOCKED
        || selected_mode == super::backend::BACKEND_MODE_BLOCKED
    {
        RUNTIME_BUDGET_MINIMAL
    } else if selected_mode == super::backend::BACKEND_MODE_FULL {
        RUNTIME_BUDGET_WIDE
    } else {
        RUNTIME_BUDGET_MEDIUM
    }
}

#[inline(always)]
pub fn runtime_dispatch_class(
    runtime_strategy: &'static str,
    runtime_budget_class: &'static str,
) -> &'static str {
    if runtime_strategy == RUNTIME_STRATEGY_EVENT_FASTPATH
        && runtime_budget_class == RUNTIME_BUDGET_WIDE
    {
        RUNTIME_DISPATCH_LATENCY_SAFE
    } else if runtime_strategy == RUNTIME_STRATEGY_CONSERVATIVE_HOLD
        || runtime_budget_class == RUNTIME_BUDGET_MINIMAL
    {
        RUNTIME_DISPATCH_CONSERVATIVE
    } else {
        RUNTIME_DISPATCH_BALANCED
    }
}

#[inline(always)]
pub fn runtime_preemption_policy(
    selected_mode: &'static str,
    runtime_budget_class: &'static str,
) -> &'static str {
    if selected_mode == super::backend::BACKEND_MODE_BLOCKED
        || runtime_budget_class == RUNTIME_BUDGET_MINIMAL
    {
        RUNTIME_PREEMPT_HOLD
    } else if selected_mode == super::backend::BACKEND_MODE_FULL {
        RUNTIME_PREEMPT_PREEMPTIBLE
    } else {
        RUNTIME_PREEMPT_COOPERATIVE
    }
}

#[inline(always)]
pub fn runtime_scheduler_lane(dispatch_class: &'static str) -> &'static str {
    match dispatch_class {
        RUNTIME_DISPATCH_LATENCY_SAFE => RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
        RUNTIME_DISPATCH_CONSERVATIVE => RUNTIME_SCHED_LANE_BACKGROUND,
        _ => RUNTIME_SCHED_LANE_BALANCED,
    }
}

#[inline(always)]
pub fn runtime_dispatch_window(
    dispatch_class: &'static str,
    preemption_policy: &'static str,
) -> &'static str {
    if dispatch_class == RUNTIME_DISPATCH_CONSERVATIVE || preemption_policy == RUNTIME_PREEMPT_HOLD
    {
        RUNTIME_DISPATCH_WINDOW_HOLD
    } else if dispatch_class == RUNTIME_DISPATCH_LATENCY_SAFE {
        RUNTIME_DISPATCH_WINDOW_SHORT
    } else {
        RUNTIME_DISPATCH_WINDOW_ADAPTIVE
    }
}
