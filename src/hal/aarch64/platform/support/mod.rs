mod backend;
mod compose;
mod lifecycle;
mod operations;
mod runtime;

use super::PlatformKind;
use super::PlatformStatus;

#[derive(Debug, Clone, Copy)]
pub(super) struct VirtPlatformStatus {
    pub backend: &'static str,
    pub backend_detail: &'static str,
    pub capability_detail: &'static str,
    pub feature_detail: &'static str,
    pub interrupt_detail: &'static str,
    pub time_detail: &'static str,
    pub runtime_step: &'static str,
    pub runtime_selected_mode: &'static str,
    pub runtime_aux_step: &'static str,
    pub runtime_operation_class: &'static str,
    pub runtime_strategy: &'static str,
    pub runtime_budget_class: &'static str,
    pub runtime_dispatch_class: &'static str,
    pub runtime_preemption_policy: &'static str,
    pub runtime_scheduler_lane: &'static str,
    pub runtime_dispatch_window: &'static str,
    pub runtime_execution_profile: &'static str,
    pub runtime_execution_profile_scope: &'static str,
    pub runtime_governor_profile: &'static str,
    pub runtime_governor_profile_scope: &'static str,
    pub runtime_governor_class: &'static str,
    pub runtime_latency_bias: &'static str,
    pub runtime_energy_bias: &'static str,
    pub runtime_blocked_by: Option<&'static str>,
    pub runtime_policy_limited_by: Option<&'static str>,
    pub policy_scope: &'static str,
    pub entry_policy_scope: &'static str,
    pub resume_policy_scope: &'static str,
    pub trap_dispatch_policy_scope: &'static str,
    pub nested_policy_scope: &'static str,
    pub time_virtualization_policy_scope: &'static str,
    pub device_passthrough_policy_scope: &'static str,
    pub entry_mode: &'static str,
    pub resume_mode: &'static str,
    pub trap_mode: &'static str,
    pub nested_mode: &'static str,
    pub time_mode: &'static str,
    pub device_passthrough_mode: &'static str,
    pub backend_mode: &'static str,
    pub operational_tier: &'static str,
    pub backend_capability_level: &'static str,
    pub control_detail: &'static str,
    pub trap_detail: &'static str,
    pub detect_state: &'static str,
    pub prepare_state: &'static str,
    pub capability_state: &'static str,
    pub feature_state: &'static str,
    pub launch_state: &'static str,
    pub resume_state: &'static str,
    pub trap_state: &'static str,
    pub execution_backend: &'static str,
    pub irq_backend: &'static str,
    pub memory_backend: &'static str,
    pub launch_path: &'static str,
    pub launch_stage: &'static str,
    pub hardware_accel: bool,
    pub prep_success_rate_per_mille: u64,
    pub blocker: &'static str,
    pub nested_ready: bool,
    pub control_plane_ready: bool,
    pub exit_tracing_ready: bool,
    pub interrupt_virtualization_ready: bool,
    pub time_virtualization_ready: bool,
    pub monitoring_ready: bool,
    pub resume_ready: bool,
    pub guest_entry_ready: bool,
    pub state_save_restore_ready: bool,
    pub trap_handling_ready: bool,
    pub observability_tier: &'static str,
    pub snapshot_ready: bool,
    pub dirty_logging_ready: bool,
    pub live_migration_ready: bool,
    pub advanced_operations_tier: &'static str,
    pub isolation_tier: &'static str,
    pub memory_isolation_ready: bool,
    pub device_passthrough_ready: bool,
    pub operational_readiness: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlatformBaseStatus {
    pub kind: PlatformKind,
    pub acpi_present: bool,
    pub dtb_present: bool,
    pub hypervisor_present: bool,
    pub gic_initialized: bool,
    pub cpu_count: usize,
    pub aps_ready: u32,
    pub timer_frequency_hz: u64,
    pub vm_launch_ready: bool,
}

impl VirtPlatformStatus {
    #[cfg_attr(not(test), allow(dead_code))]
    fn can_launch_guest(self) -> bool {
        crate::hal::common::virt::can_launch_from_readiness(self.operational_readiness)
            && self.control_plane_ready
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_resume_guest(self) -> bool {
        crate::hal::common::virt::can_resume_from_flags(self.resume_ready, self.guest_entry_ready)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_passthrough_devices(self) -> bool {
        crate::hal::common::virt::can_passthrough_from_flags(
            self.memory_isolation_ready,
            self.device_passthrough_ready,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_enable_nested(self) -> bool {
        crate::hal::common::virt::can_enable_nested_from_flags(
            self.nested_ready,
            self.control_plane_ready,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_trace_exits(self) -> bool {
        crate::hal::common::virt::can_trace_from_flags(
            self.exit_tracing_ready,
            self.monitoring_ready,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn can_virtualize_time(self) -> bool {
        crate::hal::common::virt::can_virtualize_time_from_flags(
            self.time_virtualization_ready,
            self.control_plane_ready,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn operational_profile(self) -> (&'static str, bool, bool, bool) {
        crate::hal::common::virt::operational_smoke_profile(
            self.operational_readiness,
            self.control_plane_ready,
            self.exit_tracing_ready,
            self.monitoring_ready,
            self.memory_isolation_ready,
            self.device_passthrough_ready,
        )
    }
}

#[inline(always)]
pub(super) fn classify_platform(
    acpi_present: bool,
    dtb_present: bool,
    hypervisor_present: bool,
) -> PlatformKind {
    if hypervisor_present && dtb_present {
        PlatformKind::Virt
    } else if acpi_present || dtb_present {
        PlatformKind::Server
    } else {
        PlatformKind::BareMetalUnknown
    }
}

#[inline(always)]
pub(super) fn virt_platform_status(
    virt: crate::hal::common::virt::VirtStatus,
    gic: crate::hal::aarch64::gic::GicStats,
    timer: crate::hal::aarch64::timer::GenericTimerStats,
    dtb_present: bool,
) -> VirtPlatformStatus {
    let hardware_accel = crate::hal::common::virt::hardware_accel_ready(virt);
    let effective_policy = crate::config::KernelConfig::virtualization_effective_profile();
    let scope_profile = crate::config::KernelConfig::virtualization_policy_scope_profile();
    let memory_isolation_ready = virt.vm_launch_ready && gic.initialized && dtb_present;
    let backend = crate::hal::common::virt::backend_name(virt);
    let ops = operations::current_operation_support(virt, gic, timer, memory_isolation_ready);
    let detail = crate::hal::aarch64::virt::detail::summarize(
        virt,
        gic.initialized,
        gic.version,
        timer.frequency_hz,
        memory_isolation_ready,
    );
    let backend_support =
        backend::current_backend_support(backend, hardware_accel, gic, memory_isolation_ready);
    let runtime = runtime::current_runtime_support();
    let lifecycle = crate::hal::aarch64::virt::guest_backend_state_machine();

    VirtPlatformStatus {
        backend,
        backend_detail: detail.backend_detail,
        capability_detail: detail.capability_detail,
        feature_detail: detail.feature_detail,
        interrupt_detail: detail.interrupt_detail,
        time_detail: detail.time_detail,
        runtime_step: runtime.plan.step,
        runtime_selected_mode: runtime.plan.selected_mode,
        runtime_aux_step: runtime.plan.aux_step,
        runtime_operation_class: runtime.plan.operation_class,
        runtime_strategy: runtime.plan.runtime_strategy,
        runtime_budget_class: runtime.plan.runtime_budget_class,
        runtime_dispatch_class: runtime.dispatch.dispatch_class,
        runtime_preemption_policy: runtime.dispatch.preemption_policy,
        runtime_scheduler_lane: runtime.schedule.scheduler_lane,
        runtime_dispatch_window: runtime.schedule.dispatch_window,
        runtime_execution_profile: runtime.execution_profile,
        runtime_execution_profile_scope: runtime.execution_profile_scope,
        runtime_governor_profile: runtime.governor_profile,
        runtime_governor_profile_scope: runtime.governor_profile_scope,
        runtime_governor_class: runtime.governor.governor_class,
        runtime_latency_bias: runtime.governor.latency_bias,
        runtime_energy_bias: runtime.governor.energy_bias,
        runtime_blocked_by: runtime.plan.blocked_by,
        runtime_policy_limited_by: runtime.plan.policy_limited_by,
        policy_scope: scope_profile.overall,
        entry_policy_scope: scope_profile.entry,
        resume_policy_scope: scope_profile.resume,
        trap_dispatch_policy_scope: scope_profile.trap_dispatch,
        nested_policy_scope: scope_profile.nested,
        time_virtualization_policy_scope: scope_profile.time_virtualization,
        device_passthrough_policy_scope: scope_profile.device_passthrough,
        entry_mode: detail.entry_mode,
        resume_mode: detail.resume_mode,
        trap_mode: detail.trap_mode,
        nested_mode: detail.nested_mode,
        time_mode: detail.time_mode,
        device_passthrough_mode: detail.device_passthrough_mode,
        backend_mode: detail.backend_mode,
        operational_tier: detail.operational_tier,
        backend_capability_level: detail.capability_level,
        control_detail: detail.control_detail,
        trap_detail: detail.trap_detail,
        detect_state: lifecycle.detect_state,
        prepare_state: lifecycle.prepare_state,
        capability_state: lifecycle.capability_state,
        feature_state: lifecycle.feature_state,
        launch_state: lifecycle.launch_state,
        resume_state: lifecycle.resume_state,
        trap_state: lifecycle.trap_state,
        execution_backend: backend_support.execution_backend,
        irq_backend: backend_support.irq_backend,
        memory_backend: backend_support.memory_backend,
        launch_path: backend_support.launch_path,
        launch_stage: detail.launch_stage,
        hardware_accel,
        prep_success_rate_per_mille: crate::hal::common::virt::prep_success_rate_per_mille(virt),
        blocker: virt.blocker,
        nested_ready: ops.nested_ready,
        control_plane_ready: ops.control_plane_ready,
        exit_tracing_ready: ops.exit_tracing_ready,
        interrupt_virtualization_ready: ops.interrupt_virtualization_ready,
        time_virtualization_ready: ops.time_virtualization_ready,
        monitoring_ready: ops.monitoring_ready,
        resume_ready: ops.resume_ready,
        guest_entry_ready: ops.guest_entry_ready,
        state_save_restore_ready: ops.state_save_restore_ready,
        trap_handling_ready: ops.trap_handling_ready,
        observability_tier: ops.summary.observability_tier,
        snapshot_ready: ops.snapshot_ready,
        dirty_logging_ready: ops.dirty_logging_ready,
        live_migration_ready: ops.live_migration_ready,
        advanced_operations_tier: ops.advanced_operations_tier,
        isolation_tier: detail.isolation_tier,
        memory_isolation_ready,
        device_passthrough_ready: ops.device_passthrough_ready,
        operational_readiness: detail.operational_readiness(),
    }
}

#[inline(always)]
pub(super) fn compose_platform_status(
    base: PlatformBaseStatus,
    virt_status: VirtPlatformStatus,
) -> PlatformStatus {
    compose::compose_platform_status(base, virt_status)
}

#[cfg(test)]
mod tests;
