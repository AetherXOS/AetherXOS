use crate::modules::posix::PosixErrno;

pub(super) const RLIMIT_NOFILE: i32 = crate::modules::posix_consts::process::RLIMIT_NOFILE;
pub(super) const RLIMIT_NPROC: i32 = crate::modules::posix_consts::process::RLIMIT_NPROC;
pub(super) const RLIMIT_STACK: i32 = crate::modules::posix_consts::process::RLIMIT_STACK;

#[cfg(feature = "vfs")]
#[inline(always)]
pub(super) fn basename_bytes(path: &str) -> &[u8] {
    if path.is_empty() {
        return b"exec";
    }
    let candidate = path.rsplit('/').next().unwrap_or(path);
    if candidate.is_empty() {
        b"exec"
    } else {
        candidate.as_bytes()
    }
}

#[inline(always)]
pub(super) fn clamp_nice(value: i32) -> i32 {
    value.clamp(-20, 19)
}

#[inline(always)]
pub(super) fn normalize_target_pid(current_pid: usize, requested_pid: usize) -> usize {
    if requested_pid == 0 {
        current_pid
    } else {
        requested_pid
    }
}

#[inline(always)]
pub(super) fn validate_rlimit_resource(resource: i32) -> Result<(), PosixErrno> {
    match resource {
        RLIMIT_NOFILE | RLIMIT_NPROC | RLIMIT_STACK => Ok(()),
        _ => Err(PosixErrno::Invalid),
    }
}

#[inline(always)]
pub(super) fn validate_rlimit_pair(soft: u64, hard: u64) -> Result<(), PosixErrno> {
    if soft > hard {
        Err(PosixErrno::Invalid)
    } else {
        Ok(())
    }
}

#[inline(always)]
pub(super) fn alarm_ticks_from_seconds(seconds: usize, hz: u64) -> u64 {
    (seconds as u64).saturating_mul(hz)
}

#[inline(always)]
pub(super) fn remaining_alarm_seconds(old_expire: u64, now_ticks: u64, hz: u64) -> usize {
    if old_expire > now_ticks && hz != 0 {
        ((old_expire - now_ticks) / hz) as usize
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "vfs")]
    #[test_case]
    fn basename_bytes_handles_empty_root_and_nested_paths() {
        assert_eq!(basename_bytes(""), b"exec");
        assert_eq!(basename_bytes("/"), b"exec");
        assert_eq!(basename_bytes("/bin/sh"), b"sh");
    }

    #[test_case]
    fn clamp_nice_and_target_pid_helpers_stay_stable() {
        assert_eq!(clamp_nice(-99), -20);
        assert_eq!(clamp_nice(0), 0);
        assert_eq!(clamp_nice(99), 19);
        assert_eq!(normalize_target_pid(44, 0), 44);
        assert_eq!(normalize_target_pid(44, 7), 7);
    }

    #[test_case]
    fn rlimit_validation_accepts_known_resources_and_rejects_bad_pairs() {
        assert!(validate_rlimit_resource(RLIMIT_NOFILE).is_ok());
        assert!(validate_rlimit_resource(RLIMIT_NPROC).is_ok());
        assert!(validate_rlimit_resource(RLIMIT_STACK).is_ok());
        assert_eq!(validate_rlimit_resource(-1), Err(PosixErrno::Invalid));
        assert!(validate_rlimit_pair(1, 1).is_ok());
        assert_eq!(validate_rlimit_pair(2, 1), Err(PosixErrno::Invalid));
    }

    #[test_case]
    fn alarm_helpers_saturate_and_handle_expired_entries() {
        assert_eq!(alarm_ticks_from_seconds(3, 100), 300);
        assert_eq!(remaining_alarm_seconds(500, 200, 100), 3);
        assert_eq!(remaining_alarm_seconds(100, 200, 100), 0);
        assert_eq!(remaining_alarm_seconds(500, 200, 0), 0);
    }
}
