#[cfg(all(not(feature = "linux_compat"), feature = "posix_time"))]
use crate::kernel::syscalls::linux_shim::util::read_user_pod;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_time"))]
use crate::kernel::syscalls::linux_errno;
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_shim::util::write_user_pod;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_time"))]
use crate::kernel::syscalls::with_user_write_bytes;

#[repr(C)]
#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
struct LinuxTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_clock_gettime(clock_id: usize, ts_ptr: usize) -> usize {
    #[cfg(feature = "posix_time")]
    {
        let spec = match crate::modules::posix::time::clock_gettime_raw(clock_id as i32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        let src = LinuxTimespec {
            tv_sec: spec.sec,
            tv_nsec: spec.nsec as i64,
        };
        write_user_pod(ts_ptr, &src)
            .map(|_| 0usize)
            .unwrap_or_else(|err| err)
    }
    #[cfg(not(feature = "posix_time"))]
    {
        let _ = clock_id;
        let src = LinuxTimespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        write_user_pod(ts_ptr, &src)
            .map(|_| 0usize)
            .unwrap_or_else(|err| err)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_clock_nanosleep(
    clock_id: usize,
    flags: usize,
    req_ptr: usize,
    rem_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_time")]
    {
        let req = match read_user_pod::<LinuxTimespec>(req_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let req_ts = crate::modules::posix::time::PosixTimespec {
            sec: req.tv_sec,
            nsec: if !(0..1_000_000_000).contains(&req.tv_nsec) || req.tv_nsec > i32::MAX as i64 {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            } else {
                req.tv_nsec as i32
            },
        };

        match crate::modules::posix::time::clock_nanosleep_raw(
            clock_id as i32,
            flags as i32,
            req_ts,
        ) {
            Ok(_) => 0,
            Err(err) => {
                if rem_ptr != 0 {
                    if with_user_write_bytes(
                        rem_ptr,
                        core::mem::size_of::<LinuxTimespec>(),
                        |dst| {
                            dst.fill(0);
                            0
                        },
                    )
                    .is_err()
                    {
                        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
                    }
                }
                linux_errno(err.code())
            }
        }
    }
    #[cfg(not(feature = "posix_time"))]
    {
        let _ = (clock_id, flags, req_ptr, rem_ptr);
        0
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;
    use crate::kernel::syscalls::linux_errno;

    #[test_case]
    fn clock_nanosleep_invalid_ptr_behavior() {
        #[cfg(feature = "posix_time")]
        {
            assert_eq!(
                sys_linux_clock_nanosleep(0, 0, 0, 0),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }

        #[cfg(not(feature = "posix_time"))]
        {
            assert_eq!(sys_linux_clock_nanosleep(0, 0, 0, 0), 0);
        }
    }

    #[test_case]
    fn clock_nanosleep_rejects_out_of_range_nsec() {
        #[cfg(feature = "posix_time")]
        {
            let req = LinuxTimespec {
                tv_sec: 0,
                tv_nsec: (i32::MAX as i64) + 1,
            };
            assert_eq!(
                sys_linux_clock_nanosleep(0, 0, (&req as *const LinuxTimespec) as usize, 0),
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            );
        }

        #[cfg(not(feature = "posix_time"))]
        {
            let req = LinuxTimespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            assert_eq!(
                sys_linux_clock_nanosleep(0, 0, (&req as *const LinuxTimespec) as usize, 0),
                0
            );
        }
    }

    #[test_case]
    fn clock_nanosleep_invalid_remainder_pointer_returns_efault() {
        #[cfg(feature = "posix_time")]
        {
            let req = LinuxTimespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            assert_eq!(
                sys_linux_clock_nanosleep(
                    usize::MAX,
                    0,
                    (&req as *const LinuxTimespec) as usize,
                    0x1
                ),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }
    }

    #[test_case]
    fn clock_gettime_invalid_pointer_behavior() {
        #[cfg(feature = "posix_time")]
        {
            assert_eq!(
                sys_linux_clock_gettime(0, 0x1),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }

        #[cfg(not(feature = "posix_time"))]
        {
            assert_eq!(
                sys_linux_clock_gettime(0, 0x1),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }
    }

    #[test_case]
    fn clock_nanosleep_rejects_negative_nsec() {
        #[cfg(feature = "posix_time")]
        {
            let req = LinuxTimespec {
                tv_sec: 0,
                tv_nsec: -1,
            };
            assert_eq!(
                sys_linux_clock_nanosleep(0, 0, (&req as *const LinuxTimespec) as usize, 0),
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            );
        }

        #[cfg(not(feature = "posix_time"))]
        {
            let req = LinuxTimespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            assert_eq!(
                sys_linux_clock_nanosleep(0, 0, (&req as *const LinuxTimespec) as usize, 0),
                0
            );
        }
    }
}
