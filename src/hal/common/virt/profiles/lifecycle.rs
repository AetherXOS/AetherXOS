use super::*;

#[inline(always)]
pub fn primary_lifecycle(vmx_lifecycle: &'static str, svm_lifecycle: &'static str) -> &'static str {
    for candidate in [
        LIFECYCLE_STATE_ACTIVE,
        LIFECYCLE_STATE_PREPARED,
        LIFECYCLE_STATE_FAILED,
        LIFECYCLE_STATE_TORN_DOWN,
    ] {
        if vmx_lifecycle == candidate || svm_lifecycle == candidate {
            return candidate;
        }
    }
    LIFECYCLE_STATE_UNINITIALIZED
}

#[inline(always)]
pub fn has_launch_context(lifecycle: &'static str) -> bool {
    matches!(lifecycle, LIFECYCLE_STATE_PREPARED | LIFECYCLE_STATE_ACTIVE)
}

#[inline(always)]
pub fn activate_lifecycle_code(launch_ready: bool, backend_ready: bool) -> Option<u8> {
    if launch_ready && backend_ready {
        Some(LIFECYCLE_CODE_ACTIVE)
    } else {
        None
    }
}

#[inline(always)]
pub fn reset_lifecycle_code(has_context: bool) -> Option<u8> {
    if has_context {
        Some(LIFECYCLE_CODE_PREPARED)
    } else {
        None
    }
}

#[inline(always)]
pub fn teardown_lifecycle_code(current_code: u8) -> Option<u8> {
    if current_code != LIFECYCLE_CODE_UNINITIALIZED {
        Some(LIFECYCLE_CODE_TORN_DOWN)
    } else {
        None
    }
}

#[inline(always)]
pub fn lifecycle_summary_from_states(
    policy: crate::config::VirtualizationRuntimeProfile,
    detect_state: &'static str,
    prepare_state: &'static str,
    capability_state: &'static str,
    feature_state: &'static str,
    launch_state: &'static str,
    resume_state: &'static str,
    trap_state: &'static str,
) -> &'static str {
    if !policy.trap_tracing && resume_state == READINESS_READY {
        LIFECYCLE_SUMMARY_RESUME_POLICY_LIMITED
    } else if !policy.snapshot && prepare_state == LIFECYCLE_STATE_PREPARED {
        LIFECYCLE_SUMMARY_PREPARED_POLICY_LIMITED
    } else if trap_state == LIFECYCLE_STATE_PREPARED && resume_state == READINESS_READY {
        LIFECYCLE_SUMMARY_TRAP_READY
    } else if resume_state == READINESS_READY && launch_state == LIFECYCLE_STATE_PREPARED {
        LIFECYCLE_SUMMARY_RESUME_READY
    } else if launch_state == LIFECYCLE_STATE_PREPARED {
        LIFECYCLE_SUMMARY_LAUNCH_READY
    } else if capability_state == LIFECYCLE_STATE_ACTIVE || feature_state == LIFECYCLE_STATE_ACTIVE
    {
        LIFECYCLE_SUMMARY_CAPABILITY_ACTIVE
    } else if prepare_state == LIFECYCLE_STATE_PREPARED {
        LIFECYCLE_SUMMARY_PREPARED
    } else if detect_state == LIFECYCLE_SUMMARY_DETECTED {
        LIFECYCLE_SUMMARY_DETECTED
    } else {
        LIFECYCLE_SUMMARY_BLOCKED
    }
}

#[inline(always)]
pub fn lifecycle_progress_per_mille(
    completed_steps: u16,
    policy: crate::config::VirtualizationRuntimeProfile,
) -> u16 {
    let policy_penalty = u16::from(!policy.snapshot)
        + u16::from(!policy.dirty_logging)
        + u16::from(!policy.live_migration)
        + u16::from(!policy.trap_tracing);
    let progress_per_mille = completed_steps.saturating_mul(1000) / 7;
    progress_per_mille.saturating_sub(policy_penalty * 25)
}
