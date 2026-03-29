use super::*;

#[inline(always)]
pub fn guest_lifecycle_profile(
    lifecycle: &'static str,
    launch_ready: bool,
    resume_ready: bool,
    advanced_tier: &'static str,
) -> (&'static str, bool, bool, &'static str) {
    (lifecycle, launch_ready, resume_ready, advanced_tier)
}

#[inline(always)]
pub fn guest_resume_ready(lifecycle: &'static str, launch_ready: bool, has_context: bool) -> bool {
    launch_ready && has_context && has_launch_context(lifecycle)
}

#[inline(always)]
pub fn guest_control_profile(
    control_ready: bool,
    trap_ready: bool,
    snapshot_ready: bool,
    launch_ready: bool,
) -> (&'static str, bool, bool, bool) {
    let stage = if launch_ready && control_ready && trap_ready {
        GUEST_CONTROL_READY
    } else if launch_ready && control_ready {
        GUEST_CONTROL_PARTIAL
    } else if control_ready || snapshot_ready {
        GUEST_CONTROL_PREPARED
    } else {
        GUEST_CONTROL_BLOCKED
    };
    (stage, control_ready, trap_ready, snapshot_ready)
}

#[inline(always)]
pub fn guest_runtime_profile(flags: GuestRuntimeFlags) -> (&'static str, bool, bool, bool, bool) {
    let control = guest_control_profile(
        flags.control_ready,
        flags.trap_ready,
        flags.snapshot_ready,
        flags.launch_ready,
    );
    (
        control.0,
        control.1,
        control.2,
        guest_resume_ready(control.0, flags.launch_ready, flags.resume_ready),
        control.3,
    )
}

#[inline(always)]
pub fn guest_exit_profile(flags: GuestExitFlags) -> (&'static str, bool, bool, bool, bool) {
    let stage = if flags.launch_ready && flags.trap_ready && flags.trace_ready {
        GUEST_EXIT_READY
    } else if flags.launch_ready && (flags.trap_ready || flags.trace_ready) {
        GUEST_EXIT_PARTIAL
    } else if flags.launch_ready || flags.interrupt_ready || flags.time_ready {
        GUEST_EXIT_PREPARED
    } else {
        GUEST_EXIT_BLOCKED
    };
    (
        stage,
        flags.trap_ready,
        flags.trace_ready,
        flags.interrupt_ready,
        flags.time_ready,
    )
}

#[inline(always)]
pub fn guest_launch_profile(flags: GuestLaunchFlags) -> (&'static str, bool, bool, bool) {
    let stage = if flags.launch_ready && flags.control_ready && flags.guest_entry_ready {
        GUEST_LAUNCH_READY
    } else if flags.launch_ready && (flags.control_ready || flags.guest_entry_ready) {
        GUEST_LAUNCH_PARTIAL
    } else if flags.launch_ready || flags.memory_isolation_ready {
        GUEST_LAUNCH_PREPARED
    } else {
        GUEST_LAUNCH_BLOCKED
    };
    (
        stage,
        flags.control_ready,
        flags.guest_entry_ready,
        flags.memory_isolation_ready,
    )
}

#[inline(always)]
pub fn guest_operation_profile(
    launch: GuestLaunchFlags,
    runtime: GuestRuntimeFlags,
    exit: GuestExitFlags,
) -> GuestOperationProfile {
    let launch_profile = guest_launch_profile(launch);
    let runtime_profile = guest_runtime_profile(runtime);
    let exit_profile = guest_exit_profile(exit);
    GuestOperationProfile {
        launch_stage: launch_profile.0,
        runtime_stage: runtime_profile.0,
        exit_stage: exit_profile.0,
        control_ready: runtime_profile.1,
        trap_ready: runtime_profile.2,
        guest_entry_ready: launch_profile.2,
        resume_ready: runtime_profile.3,
        snapshot_ready: runtime_profile.4,
        memory_isolation_ready: launch_profile.3,
    }
}

#[inline(always)]
pub fn control_is_operational(control_detail: &'static str) -> bool {
    control_detail != CONTROL_NONE
        && control_detail != CONTROL_DETECTED
        && control_detail != CONTROL_EL2_DETECTED
}

#[inline(always)]
pub fn trap_is_operational(trap_detail: &'static str) -> bool {
    trap_detail != TRAP_NOT_READY
}

#[inline(always)]
pub fn interrupt_is_operational(interrupt_detail: &'static str) -> bool {
    interrupt_detail != INTERRUPT_NONE && interrupt_detail != INTERRUPT_BASIC
}

#[inline(always)]
pub fn time_is_operational(time_detail: &'static str) -> bool {
    time_detail != TIME_NONE && time_detail != TIME_BASIC
}
