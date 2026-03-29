use super::*;

pub(crate) fn sys_get_power_stats(ptr: usize, len: usize) -> usize {
    SYSCALL_POWER_STATS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_STATS) {
        return err;
    }

    let pwr = crate::kernel::power::stats();
    let pstate = match pwr.current_pstate {
        crate::kernel::power::PState::HighPerf => 0,
        crate::kernel::power::PState::Balanced => 1,
        crate::kernel::power::PState::PowerSave => 2,
    };
    write_user_words(
        ptr,
        len,
        [
            pwr.idle_calls as usize,
            pwr.c1_entries as usize,
            pwr.c2_entries as usize,
            pwr.c3_entries as usize,
            pwr.pstate_switches as usize,
            pstate,
            pwr.policy_override_active as usize,
            pwr.policy_override_set_calls as usize,
            pwr.policy_override_clear_calls as usize,
            pwr.cstate_override_active as usize,
            pwr.cstate_override_set_calls as usize,
            pwr.cstate_override_clear_calls as usize,
            pwr.acpi_profile_loaded as usize,
            pwr.policy_guard_hits as usize,
            pwr.runqueue_clamp_events as usize,
            pwr.failsafe_idle_fallbacks as usize,
            pwr.override_rejects_no_acpi as usize,
        ],
    )
}

pub(crate) fn sys_set_power_override(mode: usize) -> usize {
    SYSCALL_POWER_OVERRIDE_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_CONTROL)
    {
        return err;
    }

    let Some(mode) = PowerOverrideMode::from_usize(mode) else {
        return invalid_arg();
    };

    if !crate::kernel::power::set_pstate_override_guarded(mode.to_kernel()) {
        return invalid_arg();
    }
    0
}

pub(crate) fn sys_clear_power_override() -> usize {
    SYSCALL_POWER_OVERRIDE_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_CONTROL)
    {
        return err;
    }
    crate::kernel::power::clear_pstate_override();
    0
}

pub(crate) fn sys_set_cstate_override(mode: usize) -> usize {
    SYSCALL_POWER_CSTATE_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_CONTROL)
    {
        return err;
    }

    let Some(mode) = CStateOverrideMode::from_usize(mode) else {
        return invalid_arg();
    };

    if !crate::kernel::power::set_cstate_override_guarded(mode.to_kernel()) {
        return invalid_arg();
    }
    0
}

pub(crate) fn sys_clear_cstate_override() -> usize {
    SYSCALL_POWER_CSTATE_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_CONTROL)
    {
        return err;
    }
    crate::kernel::power::clear_cstate_override();
    0
}
