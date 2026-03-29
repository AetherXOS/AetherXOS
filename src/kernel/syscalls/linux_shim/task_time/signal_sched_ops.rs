use super::*;

#[cfg(not(feature = "linux_compat"))]
fn decode_sched_policy(policy: usize) -> Result<i32, usize> {
    match policy as i32 {
        crate::modules::posix_consts::process::SCHED_OTHER
        | crate::modules::posix_consts::process::SCHED_FIFO
        | crate::modules::posix_consts::process::SCHED_RR => Ok(policy as i32),
        _ => Err(linux_errno(crate::modules::posix_consts::errno::EINVAL)),
    }
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        all(feature = "posix_signal", feature = "posix_process")
    ),
    test
))]
fn decode_signal_number(signal: usize) -> Result<i32, usize> {
    let signal = i32::try_from(signal)
        .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL))?;
    if signal == 0 || (1..=64).contains(&signal) {
        Ok(signal)
    } else {
        Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_kill(pid: usize, signal: usize) -> usize {
    #[cfg(all(feature = "posix_process", feature = "posix_signal"))]
    {
        let signal = match decode_signal_number(signal) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::signal::kill(pid, signal) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(all(feature = "posix_process", feature = "posix_signal")))]
    {
        let _ = (pid, signal);
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_tgkill(tgid: usize, tid: usize, signal: usize) -> usize {
    #[cfg(feature = "posix_signal")]
    {
        let signal = match decode_signal_number(signal) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::signal::tgkill(tgid, tid, signal) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_signal"))]
    {
        let _ = (tgid, tid, signal);
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn sys_linux_rt_sigreturn() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_sched_get_priority_max(policy: usize) -> usize {
    match decode_sched_policy(policy) {
        Ok(crate::modules::posix_consts::process::SCHED_OTHER) => 0,
        Ok(crate::modules::posix_consts::process::SCHED_FIFO)
        | Ok(crate::modules::posix_consts::process::SCHED_RR) => 99,
        Err(err) => err,
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_sched_get_priority_min(policy: usize) -> usize {
    match decode_sched_policy(policy) {
        Ok(crate::modules::posix_consts::process::SCHED_OTHER) => 0,
        Ok(crate::modules::posix_consts::process::SCHED_FIFO)
        | Ok(crate::modules::posix_consts::process::SCHED_RR) => 1,
        Err(err) => err,
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn signal_decoder_accepts_zero_and_standard_linux_range() {
        assert_eq!(decode_signal_number(0), Ok(0));
        assert_eq!(decode_signal_number(1), Ok(1));
        assert_eq!(decode_signal_number(64), Ok(64));
    }

    #[test_case]
    fn signal_decoder_rejects_out_of_range_values() {
        assert_eq!(
            decode_signal_number(65),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            decode_signal_number(usize::MAX),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn sched_policy_decoder_accepts_linux_policies() {
        assert_eq!(
            decode_sched_policy(crate::modules::posix_consts::process::SCHED_OTHER as usize),
            Ok(crate::modules::posix_consts::process::SCHED_OTHER)
        );
        assert_eq!(
            decode_sched_policy(crate::modules::posix_consts::process::SCHED_FIFO as usize),
            Ok(crate::modules::posix_consts::process::SCHED_FIFO)
        );
        assert_eq!(
            decode_sched_policy(crate::modules::posix_consts::process::SCHED_RR as usize),
            Ok(crate::modules::posix_consts::process::SCHED_RR)
        );
    }

    #[test_case]
    fn sched_priority_bounds_match_linux_conventions() {
        assert_eq!(
            sys_linux_sched_get_priority_max(
                crate::modules::posix_consts::process::SCHED_OTHER as usize
            ),
            0
        );
        assert_eq!(
            sys_linux_sched_get_priority_min(
                crate::modules::posix_consts::process::SCHED_OTHER as usize
            ),
            0
        );
        assert_eq!(
            sys_linux_sched_get_priority_max(
                crate::modules::posix_consts::process::SCHED_FIFO as usize
            ),
            99
        );
        assert_eq!(
            sys_linux_sched_get_priority_min(
                crate::modules::posix_consts::process::SCHED_FIFO as usize
            ),
            1
        );
        assert_eq!(
            sys_linux_sched_get_priority_max(
                crate::modules::posix_consts::process::SCHED_RR as usize
            ),
            99
        );
        assert_eq!(
            sys_linux_sched_get_priority_min(
                crate::modules::posix_consts::process::SCHED_RR as usize
            ),
            1
        );
    }

    #[test_case]
    fn sched_priority_rejects_unknown_policy() {
        assert_eq!(
            sys_linux_sched_get_priority_max(usize::MAX),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_sched_get_priority_min(usize::MAX),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn rt_sigreturn_reports_inval_in_shim_mode() {
        assert_eq!(
            sys_linux_rt_sigreturn(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn kill_and_tgkill_reject_invalid_signal_numbers_before_backend_lookup() {
        assert_eq!(
            sys_linux_kill(1, 65),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_tgkill(1, 1, 65),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
