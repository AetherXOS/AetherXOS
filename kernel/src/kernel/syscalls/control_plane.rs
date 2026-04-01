use super::*;

pub(super) fn sys_get_lottery_replay_latest(ptr: usize, len: usize) -> usize {
    SYSCALL_LOTTERY_REPLAY_LATEST_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "sched_lottery")]
    {
        let Some(ev) = crate::modules::schedulers::lottery::latest_replay_event() else {
            return 0;
        };
        with_user_write_words(ptr, len, LOTTERY_REPLAY_LATEST_WORDS, |out| {
            out[0] = ev.seq as usize;
            out[1] = ev.task_id.0;
            out[2] = ev.winner_ticket as usize;
            out[3] = ev.total_tickets as usize;
            out[4] = ev.rng_state as usize;
            required_bytes(LOTTERY_REPLAY_LATEST_WORDS)
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "sched_lottery"))]
    {
        let _ = (ptr, len);
        0
    }
}

pub(super) fn sys_set_policy_drift_control(
    sample_interval_ticks: usize,
    cooldown_ticks: usize,
) -> usize {
    SYSCALL_POLICY_DRIFT_CONTROL_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_CONTROL)
    {
        return err;
    }

    let sample_override = if sample_interval_ticks == 0 {
        None
    } else {
        Some(sample_interval_ticks as u64)
    };
    let cooldown_override = if cooldown_ticks == 0 {
        None
    } else {
        Some(cooldown_ticks as u64)
    };

    crate::config::KernelConfig::set_runtime_policy_drift_sample_interval_ticks(sample_override);
    crate::config::KernelConfig::set_runtime_policy_drift_reapply_cooldown_ticks(cooldown_override);
    0
}

pub(super) fn sys_get_policy_drift_control(ptr: usize, len: usize) -> usize {
    SYSCALL_POLICY_DRIFT_CONTROL_GET_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_STATS) {
        return err;
    }

    write_user_words(
        ptr,
        len,
        [
            crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks() as usize,
            crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks() as usize,
        ],
    )
}

pub(super) fn sys_get_policy_drift_reason_text(
    reason_code: usize,
    ptr: usize,
    len: usize,
) -> usize {
    SYSCALL_POLICY_DRIFT_REASON_TEXT_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_STATS) {
        return err;
    }

    let Ok(reason) = u8::try_from(reason_code) else {
        return invalid_arg();
    };
    let text = crate::kernel::policy::drift_reason_name(reason).as_bytes();
    let needed = text.len().saturating_add(1);

    with_user_write_bytes(ptr, len, |dst| {
        if dst.len() < needed {
            return invalid_arg();
        }
        dst[..text.len()].copy_from_slice(text);
        dst[text.len()] = 0;
        needed
    })
    .unwrap_or_else(|err| err)
}

#[path = "control_plane_ops.rs"]
mod control_plane_ops;
pub(crate) use control_plane_ops::*;
