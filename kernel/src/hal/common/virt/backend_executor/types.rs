#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationExecutionPolicy {
    pub runtime: crate::config::VirtualizationRuntimeProfile,
    pub cargo: crate::config::VirtualizationRuntimeProfile,
    pub effective: crate::config::VirtualizationRuntimeProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBackendExecution {
    pub backend_family: &'static str,
    pub backend_detail: &'static str,
    pub capability_detail: &'static str,
    pub feature_detail: &'static str,
    pub transition_stage: &'static str,
    pub selected_phase: &'static str,
    pub selected_action: &'static str,
    pub operational_path: &'static str,
    pub ready: bool,
    pub blocked_by: Option<&'static str>,
    pub policy: VirtualizationExecutionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBackendRuntimePlan {
    pub backend_family: &'static str,
    pub operational_path: &'static str,
    pub transition_stage: &'static str,
    pub step: &'static str,
    pub aux_step: &'static str,
    pub operation_class: &'static str,
    pub selected_mode: &'static str,
    pub runtime_strategy: &'static str,
    pub runtime_budget_class: &'static str,
    pub ready: bool,
    pub blocked_by: Option<&'static str>,
    pub policy_limited_by: Option<&'static str>,
    pub policy: VirtualizationExecutionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRuntimeDispatchHint {
    pub runtime_strategy: &'static str,
    pub runtime_budget_class: &'static str,
    pub dispatch_class: &'static str,
    pub preemption_policy: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRuntimeSchedulingProfile {
    pub scheduler_lane: &'static str,
    pub dispatch_window: &'static str,
    pub dispatch_class: &'static str,
    pub preemption_policy: &'static str,
}
