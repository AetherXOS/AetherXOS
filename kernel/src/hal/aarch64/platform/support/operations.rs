#[derive(Debug, Clone, Copy)]
pub(super) struct OperationSupportSnapshot {
    pub control_plane_ready: bool,
    pub exit_tracing_ready: bool,
    pub interrupt_virtualization_ready: bool,
    pub time_virtualization_ready: bool,
    pub monitoring_ready: bool,
    pub resume_ready: bool,
    pub guest_entry_ready: bool,
    pub state_save_restore_ready: bool,
    pub trap_handling_ready: bool,
    pub device_passthrough_ready: bool,
    pub nested_ready: bool,
    pub summary: crate::hal::common::virt::VirtOperationSummary,
    pub snapshot_ready: bool,
    pub dirty_logging_ready: bool,
    pub live_migration_ready: bool,
    pub advanced_operations_tier: &'static str,
}

#[inline(always)]
pub(super) fn current_operation_support(
    virt: crate::hal::common::virt::VirtStatus,
    gic: crate::hal::aarch64::gic::GicStats,
    timer: crate::hal::aarch64::timer::GenericTimerStats,
    memory_isolation_ready: bool,
) -> OperationSupportSnapshot {
    let effective_policy = crate::config::KernelConfig::virtualization_effective_profile();
    let hardware_accel = crate::hal::common::virt::hardware_accel_ready(virt);
    let exit_tracing_ready_base = virt.vm_launch_ready && gic.initialized;
    let control_plane_ready = virt.vm_launch_ready && hardware_accel;
    let interrupt_virtualization_ready = hardware_accel && gic.initialized;
    let time_virtualization_ready =
        effective_policy.time_virtualization && hardware_accel && timer.frequency_hz != 0;
    let monitoring_ready = control_plane_ready && interrupt_virtualization_ready;
    let resume_ready = virt.vm_launch_ready && gic.initialized && timer.frequency_hz != 0;
    let guest_entry_ready = hardware_accel && resume_ready;
    let state_save_restore_ready = control_plane_ready && timer.frequency_hz != 0;
    let trap_handling_ready = interrupt_virtualization_ready && state_save_restore_ready;
    let device_passthrough_ready =
        effective_policy.device_passthrough && memory_isolation_ready && gic.version >= 3;
    let exit_tracing_ready = crate::config::KernelConfig::virtualization_trap_tracing_enabled()
        && exit_tracing_ready_base;
    let summary = crate::hal::common::virt::summarize_operations(
        crate::hal::common::virt::VirtOperationFlags {
            control_plane_ready,
            exit_tracing_ready,
            interrupt_virtualization_ready,
            time_virtualization_ready,
            monitoring_ready,
            resume_ready,
            guest_entry_ready,
            state_save_restore_ready,
            trap_handling_ready,
            memory_isolation_ready,
            device_passthrough_ready,
        },
    );
    let snapshot_ready = effective_policy.snapshot && summary.snapshot_ready;
    let dirty_logging_ready = effective_policy.dirty_logging && summary.dirty_logging_ready;
    let live_migration_ready = effective_policy.live_migration && summary.live_migration_ready;
    let advanced_operations_tier = if snapshot_ready || dirty_logging_ready || live_migration_ready
    {
        summary.advanced_operations_tier
    } else {
        "disabled"
    };

    OperationSupportSnapshot {
        control_plane_ready,
        exit_tracing_ready,
        interrupt_virtualization_ready,
        time_virtualization_ready,
        monitoring_ready,
        resume_ready,
        guest_entry_ready,
        state_save_restore_ready,
        trap_handling_ready,
        device_passthrough_ready,
        nested_ready: effective_policy.nested
            && virt.vm_launch_ready
            && gic.initialized
            && timer.frequency_hz != 0,
        summary,
        snapshot_ready,
        dirty_logging_ready,
        live_migration_ready,
        advanced_operations_tier,
    }
}
