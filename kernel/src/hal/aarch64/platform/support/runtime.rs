#[derive(Debug, Clone, Copy)]
pub(super) struct RuntimeSupportSnapshot {
    pub plan: crate::hal::common::virt::GuestBackendRuntimePlan,
    pub dispatch: crate::hal::common::virt::GuestRuntimeDispatchHint,
    pub schedule: crate::hal::common::virt::GuestRuntimeSchedulingProfile,
    pub execution_profile: &'static str,
    pub execution_profile_scope: &'static str,
    pub governor_profile: &'static str,
    pub governor_profile_scope: &'static str,
    pub governor: crate::hal::common::virt::VirtualizationRuntimeGovernor,
}

#[inline(always)]
pub(super) fn current_runtime_support() -> RuntimeSupportSnapshot {
    let plan = crate::hal::aarch64::virt::guest_backend_runtime_plan();
    let dispatch = crate::hal::aarch64::virt::guest_runtime_dispatch_hint();
    let schedule = crate::hal::aarch64::virt::guest_runtime_scheduling_profile();
    let execution_profile =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let governor_profile = crate::config::KernelConfig::virtualization_effective_governor_profile();
    let governor = crate::hal::common::virt::virtualization_runtime_governor(
        execution_profile.scheduling_class.as_str(),
        schedule.scheduler_lane,
        plan.selected_mode,
        dispatch.dispatch_class,
    );

    RuntimeSupportSnapshot {
        plan,
        dispatch,
        schedule,
        execution_profile: execution_profile.scheduling_class.as_str(),
        execution_profile_scope: crate::config::KernelConfig::virtualization_execution_policy_scope(
        ),
        governor_profile: governor_profile.governor_class.as_str(),
        governor_profile_scope: crate::config::KernelConfig::virtualization_governor_policy_scope(),
        governor,
    }
}
