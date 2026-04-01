use super::super::*;

const ITIMER_REAL: usize = 0;
const CLOCK_REALTIME_ID: i32 = crate::modules::posix_consts::time::CLOCK_REALTIME;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const NANOS_PER_MICROSECOND: i64 = 1_000;

#[inline]
fn nanos_i64_to_i32(nsec: i64) -> Result<i32, usize> {
    if nsec < i32::MIN as i64 || nsec > i32::MAX as i64 {
        return Err(linux_inval());
    }
    Ok(nsec as i32)
}

pub fn sys_linux_clock_getres(clock_id: usize, res: UserPtr<LinuxTimespec>) -> usize {
    crate::require_posix_time!((clock_id, res) => {
        let ts = match crate::modules::posix::time::clock_getres_raw(clock_id as i32) {
                    Ok(ts) => ts,
                    Err(e) => return linux_errno(e.code()),
                };
                if !res.is_null() {
                    let src = LinuxTimespec {
                        tv_sec: ts.sec,
                        tv_nsec: ts.nsec as i64,
                    };
                    if let Err(e) = res.write(&src) { return e; }
                }
                0
    })
}

pub fn sys_linux_clock_gettime(clock_id: usize, ts_ptr: UserPtr<LinuxTimespec>) -> usize {
    crate::require_posix_time!((clock_id, ts_ptr) => {
        let spec = match crate::modules::posix::time::clock_gettime_raw(clock_id as i32) {
                    Ok(v) => v,
                    Err(err) => return linux_errno(err.code()),
                };

                let src = LinuxTimespec {
                    tv_sec: spec.sec,
                    tv_nsec: spec.nsec as i64,
                };
                ts_ptr.write(&src).map(|_| 0usize).unwrap_or_else(|e| e)
    })
}

pub fn sys_linux_clock_nanosleep(
    clock_id: usize,
    flags: usize,
    req_ptr: UserPtr<LinuxTimespec>,
    rem_ptr: UserPtr<LinuxTimespec>,
) -> usize {
    crate::require_posix_time!((clock_id, flags, req_ptr, rem_ptr) => {
        let req = match req_ptr.read() {
                    Ok(v) => v,
                    Err(e) => return e,
                };

                let req_ts = crate::modules::posix::time::PosixTimespec {
                    sec: req.tv_sec,
                    nsec: match nanos_i64_to_i32(req.tv_nsec) {
                        Ok(v) => v,
                        Err(e) => return e,
                    },
                };

                match crate::modules::posix::time::clock_nanosleep_raw(clock_id as i32, flags as i32, req_ts) {
                    Ok(_) => 0,
                    Err(err) => {
                        if !rem_ptr.is_null() {
                            let _ = rem_ptr.write(&LinuxTimespec { tv_sec: 0, tv_nsec: 0 });
                        }
                        linux_errno(err.code())
                    }
                }
    })
}

pub fn sys_linux_getitimer(which: usize, curr_value: UserPtr<LinuxITimerVal>) -> usize {
    crate::require_posix_time!((which, curr_value) => {
        if which != ITIMER_REAL {
                    return linux_inval();
                }
                let pid = crate::modules::posix::process::getpid();
                let (interval, value) = crate::modules::posix::time::getitimer(pid);

                if !curr_value.is_null() {
                    let val = LinuxITimerVal {
                        it_interval: LinuxTimeval { tv_sec: interval.sec, tv_usec: interval.usec as i64 },
                        it_value: LinuxTimeval { tv_sec: value.sec, tv_usec: value.usec as i64 },
                    };
                    if let Err(e) = curr_value.write(&val) { return e; }
                }
                0
    })
}

pub fn sys_linux_setitimer(
    which: usize,
    new_value: UserPtr<LinuxITimerVal>,
    old_value: UserPtr<LinuxITimerVal>,
) -> usize {
    crate::require_posix_time!((which, new_value, old_value) => {
        if which != ITIMER_REAL {
                    return linux_inval();
                }

                let new_ts_val = match new_value.read() {
                    Ok(v) => (
                        crate::modules::posix::time::PosixTimeval { sec: v.it_interval.tv_sec, usec: v.it_interval.tv_usec as i32 },
                        crate::modules::posix::time::PosixTimeval { sec: v.it_value.tv_sec, usec: v.it_value.tv_usec as i32 }
                    ),
                    Err(e) => return e,
                };

                let pid = crate::modules::posix::process::getpid();
                let (old_interval, old_value_val) = crate::modules::posix::time::setitimer(pid, new_ts_val.0, new_ts_val.1);

                if !old_value.is_null() {
                    let val = LinuxITimerVal {
                        it_interval: LinuxTimeval { tv_sec: old_interval.sec, tv_usec: old_interval.usec as i64 },
                        it_value: LinuxTimeval { tv_sec: old_value_val.sec, tv_usec: old_value_val.usec as i64 },
                    };
                    let _ = old_value.write(&val);
                }
                0
    })
}

pub fn sys_linux_gettimeofday(tv_ptr: usize, tz_ptr: usize) -> usize {
    crate::require_posix_time!((tv_ptr, tz_ptr) => {
        let _ = tz_ptr;
                match crate::modules::posix::time::clock_gettime_raw(CLOCK_REALTIME_ID) {
                    Ok(spec) => {
                        if tv_ptr != 0 {
                            let tv = LinuxTimeval {
                                tv_sec: spec.sec,
                                tv_usec: (spec.nsec as i64 / NANOS_PER_MICROSECOND),
                            };
                            let _ = UserPtr::<LinuxTimeval>::new(tv_ptr).write(&tv);
                        }
                        0
                    }
                    Err(err) => linux_errno(err.code()),
                }
    })
}

pub fn sys_linux_alarm(seconds: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::alarm(seconds)
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = seconds;
        0
    }
}

pub fn sys_linux_clock_settime(clock_id: usize, ts_ptr: UserPtr<LinuxTimespec>) -> usize {
    crate::require_posix_time!((clock_id, ts_ptr) => {
        let ts = match ts_ptr.read() {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let pts = crate::modules::posix::time::PosixTimespec {
                    sec: ts.tv_sec,
                    nsec: match nanos_i64_to_i32(ts.tv_nsec) {
                        Ok(v) => v,
                        Err(e) => return e,
                    },
                };
                match crate::modules::posix::time::clock_settime_raw(clock_id as i32, pts) {
                    Ok(_) => 0,
                    Err(e) => linux_errno(e.code()),
                }
    })
}

pub fn sys_linux_settimeofday(tv_ptr: UserPtr<LinuxTimeval>, tz_ptr: UserPtr<u8>) -> usize {
    crate::require_posix_time!((tv_ptr, tz_ptr) => {
        let _ = tz_ptr;
                if tv_ptr.is_null() { return 0; }

                let tv = match tv_ptr.read() {
                    Ok(v) => v,
                    Err(e) => return e,
                };

                let pts = crate::modules::posix::time::PosixTimespec {
                    sec: tv.tv_sec,
                    nsec: match nanos_i64_to_i32(tv.tv_usec.saturating_mul(NANOS_PER_MICROSECOND)) {
                        Ok(v) => v,
                        Err(e) => return e,
                    },
                };

                match crate::modules::posix::time::clock_settime_raw(CLOCK_REALTIME_ID, pts) {
                    Ok(_) => 0,
                    Err(e) => linux_errno(e.code()),
                }
    })
}

pub fn sys_linux_time(tloc: UserPtr<i64>) -> usize {
    let now = crate::kernel::watchdog::global_tick();
    let total_ns = now * crate::config::KernelConfig::time_slice();
    let sec = (total_ns / NANOS_PER_SECOND) as i64;

    if !tloc.is_null() {
        let _ = tloc.write(&sec);
    }
    sec as usize
}

pub fn sys_linux_nanosleep(
    req_ptr: UserPtr<LinuxTimespec>,
    rem_ptr: UserPtr<LinuxTimespec>,
) -> usize {
    crate::require_posix_time!((req_ptr, rem_ptr) => {
        let req = match req_ptr.read() { Ok(v) => v, Err(e) => return e };
                let ts = crate::modules::posix::time::PosixTimespec {
                    sec: req.tv_sec,
                    nsec: match nanos_i64_to_i32(req.tv_nsec) {
                        Ok(v) => v,
                        Err(e) => return e,
                    },
                };
                match crate::modules::posix::time::nanosleep(ts) {
                    Ok(()) => {
                        if !rem_ptr.is_null() {
                            let _ = rem_ptr.write(&LinuxTimespec { tv_sec: 0, tv_nsec: 0 });
                        }
                        0
                    }
                    Err(e) => linux_errno(e.code()),
                }
    })
}
