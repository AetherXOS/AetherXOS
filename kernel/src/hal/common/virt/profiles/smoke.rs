use super::*;

#[inline(always)]
pub fn smoke_profile(
    status: VirtStatus,
    launch_stage: &'static str,
    capability_level: &'static str,
) -> (&'static str, bool, u64, &'static str, bool) {
    let readiness = operational_readiness_from_stage(launch_stage, capability_level);
    (
        backend_name(status),
        hardware_accel_ready(status),
        prep_success_rate_per_mille(status),
        readiness,
        can_launch_from_readiness(readiness),
    )
}

#[inline(always)]
pub fn operational_smoke_profile(
    readiness: &'static str,
    control_plane_ready: bool,
    exit_tracing_ready: bool,
    monitoring_ready: bool,
    memory_isolation_ready: bool,
    device_passthrough_ready: bool,
) -> (&'static str, bool, bool, bool) {
    (
        readiness,
        can_launch_from_readiness(readiness) && control_plane_ready,
        can_trace_from_flags(exit_tracing_ready, monitoring_ready),
        can_passthrough_from_flags(memory_isolation_ready, device_passthrough_ready),
    )
}

#[inline(always)]
pub fn summarize_operations(flags: VirtOperationFlags) -> VirtOperationSummary {
    let observability_tier =
        observability_tier_from_flags(flags.monitoring_ready, flags.trap_handling_ready);
    let snapshot_ready =
        snapshot_ready_from_flags(flags.state_save_restore_ready, flags.monitoring_ready);
    let dirty_logging_ready =
        dirty_logging_ready_from_flags(flags.trap_handling_ready, flags.memory_isolation_ready);
    let live_migration_ready = live_migration_ready_from_flags(
        snapshot_ready,
        flags.time_virtualization_ready,
        dirty_logging_ready,
    );
    let advanced_operations_tier =
        advanced_operations_tier(snapshot_ready, live_migration_ready, dirty_logging_ready);

    VirtOperationSummary {
        observability_tier,
        snapshot_ready,
        dirty_logging_ready,
        live_migration_ready,
        advanced_operations_tier,
    }
}
