use super::*;

#[inline(always)]
pub fn prep_success_state(attempts: u64, success: u64, failures: u64) -> (u64, u64, u64) {
    (attempts, success, failures)
}

#[inline(always)]
pub fn prep_success_rate_per_mille(status: VirtStatus) -> u64 {
    if status.prep_attempts == 0 {
        0
    } else {
        status.prep_success.saturating_mul(1000) / status.prep_attempts
    }
}

#[inline(always)]
pub fn hardware_accel_ready(status: VirtStatus) -> bool {
    (status.caps.vmx
        && status.enabled.vmx_enabled
        && status.enabled.vmxon_active
        && status.vmx_vmcs_ready)
        || (status.caps.svm && status.enabled.svm_enabled && status.svm_vmcb_ready)
        || (!status.caps.vmx
            && !status.caps.svm
            && status.caps.hypervisor_present
            && status.vm_launch_ready)
}

#[inline(always)]
pub fn backend_name(status: VirtStatus) -> &'static str {
    if status.caps.vmx {
        "vmx"
    } else if status.caps.svm {
        "svm"
    } else if status.caps.hypervisor_present {
        "el2"
    } else {
        "none"
    }
}

#[inline(always)]
pub fn operational_readiness_from_stage(
    launch_stage: &'static str,
    capability_level: &'static str,
) -> &'static str {
    if launch_stage == STAGE_GUEST_RUNNABLE
        && (capability_level == CAP_TIER_3 || capability_level == CAP_TIER_2)
    {
        READINESS_READY
    } else if launch_stage == STAGE_LAUNCH_PREPARED || launch_stage == STAGE_CONTROL_PLANE_READY {
        READINESS_STAGED
    } else if launch_stage == STAGE_HARDWARE_ENABLED {
        READINESS_PARTIAL
    } else {
        READINESS_BLOCKED
    }
}

#[inline(always)]
pub fn can_launch_from_readiness(readiness: &'static str) -> bool {
    matches!(readiness, READINESS_READY | READINESS_STAGED)
}

#[inline(always)]
pub fn can_trace_from_flags(exit_tracing_ready: bool, monitoring_ready: bool) -> bool {
    exit_tracing_ready && monitoring_ready
}

#[inline(always)]
pub fn can_passthrough_from_flags(
    memory_isolation_ready: bool,
    device_passthrough_ready: bool,
) -> bool {
    memory_isolation_ready && device_passthrough_ready
}

#[inline(always)]
pub fn can_resume_from_flags(resume_ready: bool, guest_entry_ready: bool) -> bool {
    resume_ready && guest_entry_ready
}

#[inline(always)]
pub fn can_enable_nested_from_flags(nested_ready: bool, control_plane_ready: bool) -> bool {
    nested_ready && control_plane_ready
}

#[inline(always)]
pub fn can_virtualize_time_from_flags(
    time_virtualization_ready: bool,
    control_plane_ready: bool,
) -> bool {
    time_virtualization_ready && control_plane_ready
}

#[inline(always)]
pub fn observability_tier_from_flags(
    monitoring_ready: bool,
    trap_handling_ready: bool,
) -> &'static str {
    if monitoring_ready && trap_handling_ready {
        OBS_TIER_FULL
    } else if monitoring_ready {
        OBS_TIER_PARTIAL
    } else {
        OBS_TIER_MINIMAL
    }
}

#[inline(always)]
pub fn snapshot_ready_from_flags(state_save_restore_ready: bool, monitoring_ready: bool) -> bool {
    state_save_restore_ready && monitoring_ready
}

#[inline(always)]
pub fn dirty_logging_ready_from_flags(
    trap_handling_ready: bool,
    memory_isolation_ready: bool,
) -> bool {
    trap_handling_ready && memory_isolation_ready
}

#[inline(always)]
pub fn live_migration_ready_from_flags(
    snapshot_ready: bool,
    time_virtualization_ready: bool,
    dirty_logging_ready: bool,
) -> bool {
    snapshot_ready && time_virtualization_ready && dirty_logging_ready
}

#[inline(always)]
pub fn advanced_operations_tier(
    snapshot_ready: bool,
    live_migration_ready: bool,
    dirty_logging_ready: bool,
) -> &'static str {
    if snapshot_ready && live_migration_ready && dirty_logging_ready {
        ADVANCED_TIER_HYPERVISOR_GRADE
    } else if snapshot_ready || dirty_logging_ready {
        ADVANCED_TIER_ADVANCED
    } else {
        ADVANCED_TIER_BASELINE
    }
}

#[inline(always)]
pub fn advanced_operations_profile(
    snapshot_ready: bool,
    dirty_logging_ready: bool,
    live_migration_ready: bool,
) -> (bool, bool, bool, &'static str) {
    (
        snapshot_ready,
        dirty_logging_ready,
        live_migration_ready,
        advanced_operations_tier(snapshot_ready, live_migration_ready, dirty_logging_ready),
    )
}

#[inline(always)]
pub fn backend_has_full_runtime(backend_detail: &'static str) -> bool {
    matches!(
        backend_detail,
        BACKEND_VMX_ACTIVE | BACKEND_SVM_ACTIVE | BACKEND_EL2_FULL
    )
}

#[inline(always)]
pub fn capability_has_entry_path(capability_detail: &'static str) -> bool {
    matches!(
        capability_detail,
        CAPABILITY_VMX_ACTIVE
            | CAPABILITY_SVM_ACTIVE
            | CAPABILITY_EL2_ACTIVE
            | CAPABILITY_EL2_ENABLED
    )
}
