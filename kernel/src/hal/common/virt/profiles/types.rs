#[derive(Debug, Clone, Copy)]
pub struct VirtOperationFlags {
    pub control_plane_ready: bool,
    pub exit_tracing_ready: bool,
    pub interrupt_virtualization_ready: bool,
    pub time_virtualization_ready: bool,
    pub monitoring_ready: bool,
    pub resume_ready: bool,
    pub guest_entry_ready: bool,
    pub state_save_restore_ready: bool,
    pub trap_handling_ready: bool,
    pub memory_isolation_ready: bool,
    pub device_passthrough_ready: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct VirtOperationSummary {
    pub observability_tier: &'static str,
    pub snapshot_ready: bool,
    pub dirty_logging_ready: bool,
    pub live_migration_ready: bool,
    pub advanced_operations_tier: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct GuestRuntimeFlags {
    pub launch_ready: bool,
    pub control_ready: bool,
    pub trap_ready: bool,
    pub resume_ready: bool,
    pub snapshot_ready: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GuestExitFlags {
    pub launch_ready: bool,
    pub trap_ready: bool,
    pub trace_ready: bool,
    pub interrupt_ready: bool,
    pub time_ready: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GuestLaunchFlags {
    pub launch_ready: bool,
    pub control_ready: bool,
    pub guest_entry_ready: bool,
    pub memory_isolation_ready: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GuestOperationProfile {
    pub launch_stage: &'static str,
    pub runtime_stage: &'static str,
    pub exit_stage: &'static str,
    pub control_ready: bool,
    pub trap_ready: bool,
    pub guest_entry_ready: bool,
    pub resume_ready: bool,
    pub snapshot_ready: bool,
    pub memory_isolation_ready: bool,
}
