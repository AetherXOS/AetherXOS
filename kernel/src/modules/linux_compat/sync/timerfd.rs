use super::super::*;

/// `timerfd_create(2)`, `timerfd_settime(2)`, `timerfd_gettime(2)`.
pub mod timerfd_flags {
    pub const TFD_TIMER_ABSTIME: usize = 0x1;
    pub const TFD_CLOEXEC: usize = 0x0008_0000;
    pub const TFD_NONBLOCK: usize = 0x0000_0800;
}

const TIMERFD_ALLOWED_FLAGS: usize =
    timerfd_flags::TFD_TIMER_ABSTIME | timerfd_flags::TFD_CLOEXEC | timerfd_flags::TFD_NONBLOCK;
const TIMERFD_MAX_CLOCK_ID: usize = crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize;
const NANOS_PER_SECOND_I64: i64 = 1_000_000_000;

#[inline]
fn zero_itimerspec() -> LinuxItimerspec {
    LinuxItimerspec {
        it_interval: LinuxTimespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: LinuxTimespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
    }
}

#[inline]
fn validate_itimerspec(spec: &LinuxItimerspec) -> bool {
    let interval_ok = spec.it_interval.tv_sec >= 0
        && (0..NANOS_PER_SECOND_I64).contains(&spec.it_interval.tv_nsec);
    let value_ok =
        spec.it_value.tv_sec >= 0 && (0..NANOS_PER_SECOND_I64).contains(&spec.it_value.tv_nsec);
    interval_ok && value_ok
}

pub fn sys_linux_timerfd_create(clockid: usize, flags: usize) -> usize {
    if clockid > TIMERFD_MAX_CLOCK_ID {
        return linux_inval();
    }
    if (flags & !TIMERFD_ALLOWED_FLAGS) != 0 {
        return linux_inval();
    }

    crate::require_posix_fs!( (clockid, flags) => {
        match crate::modules::posix::fs::openat(1, "/", "timerfd", true) {
            Ok(fd) => {
                if flags & timerfd_flags::TFD_NONBLOCK != 0 {
                    if let Err(e) = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    ) {
                        let _ = crate::modules::posix::fs::close(fd);
                        return linux_errno(e.code());
                    }
                }
                if flags & timerfd_flags::TFD_CLOEXEC != 0 {
                    super::super::fs::io::linux_fd_set_descriptor_flags(
                        fd,
                        super::super::fs::io::LINUX_FD_CLOEXEC,
                    );
                } else {
                    super::super::fs::io::linux_fd_clear_descriptor_flags(fd);
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_timerfd_settime(
    _fd: Fd,
    flags: usize,
    new_value: UserPtr<LinuxItimerspec>,
    old_value: UserPtr<LinuxItimerspec>,
) -> usize {
    crate::require_posix_fs!((_fd, flags, new_value, old_value) => {
        if (flags & !timerfd_flags::TFD_TIMER_ABSTIME) != 0 {
            return linux_inval();
        }
        let it = match new_value.read() { Ok(v) => v, Err(e) => return e };
        if !validate_itimerspec(&it) {
            return linux_inval();
        }
        if !old_value.is_null() {
            let prev = zero_itimerspec();
            if let Err(e) = old_value.write(&prev) {
                return e;
            }
        }

        0
    })
}

pub fn sys_linux_timerfd_gettime(_fd: Fd, curr_value: UserPtr<LinuxItimerspec>) -> usize {
    crate::require_posix_fs!((_fd, curr_value) => {
        let current = zero_itimerspec();
        match curr_value.write(&current) {
            Ok(_) => 0,
            Err(e) => e,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn timerfd_create_sets_cloexec_and_nonblock_visibility() {
        let fd = sys_linux_timerfd_create(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize,
            timerfd_flags::TFD_CLOEXEC | timerfd_flags::TFD_NONBLOCK,
        ) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
        assert_eq!(
            crate::modules::posix::fs::fcntl_get_status_flags(fd).expect("status flags")
                & crate::modules::posix_consts::net::O_NONBLOCK,
            crate::modules::posix_consts::net::O_NONBLOCK
        );
    }
}
