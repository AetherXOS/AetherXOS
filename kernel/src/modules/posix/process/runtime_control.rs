use super::*;

pub(super) fn getrlimit(resource: i32) -> Result<(u64, u64), PosixErrno> {
    validate_rlimit_resource(resource)?;
    let table = RLIMIT_TABLE.lock();
    Ok(*table.get(&resource).unwrap_or(&(u64::MAX, u64::MAX)))
}

pub(super) fn setrlimit(resource: i32, soft: u64, hard: u64) -> Result<(), PosixErrno> {
    validate_rlimit_pair(soft, hard)?;
    validate_rlimit_resource(resource)?;
    RLIMIT_TABLE.lock().insert(resource, (soft, hard));
    Ok(())
}

pub(super) fn prlimit(
    pid: usize,
    resource: i32,
    new: Option<(u64, u64)>,
) -> Result<(u64, u64), PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }

    let old = getrlimit(resource)?;
    if let Some((soft, hard)) = new {
        setrlimit(resource, soft, hard)?;
    }
    Ok(old)
}

pub(super) fn sched_getscheduler(pid: usize) -> Result<i32, PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }
    Ok(SCHED_POLICY.load(Ordering::Relaxed) as i32)
}

pub(super) fn sched_setscheduler(pid: usize, policy: i32, priority: i32) -> Result<(), PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }

    match policy {
        crate::modules::posix_consts::process::SCHED_OTHER
        | crate::modules::posix_consts::process::SCHED_FIFO
        | crate::modules::posix_consts::process::SCHED_RR => {}
        _ => return Err(PosixErrno::Invalid),
    }

    SCHED_POLICY.store(policy as u32, Ordering::Relaxed);
    setpriority(target, priority)
}

pub(super) fn sched_getparam(pid: usize) -> Result<i32, PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    getpriority(target)
}

pub(super) fn sched_setparam(pid: usize, priority: i32) -> Result<(), PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    setpriority(target, priority)
}

pub(super) fn getcpu() -> Result<(u32, u32), PosixErrno> {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() }.ok_or(PosixErrno::Invalid)?;
    Ok((cpu.cpu_id.0 as u32, 0))
}

pub(super) fn getrusage(who: i32) -> Result<PosixRusage, PosixErrno> {
    match who {
        crate::modules::posix_consts::process::RUSAGE_SELF
        | crate::modules::posix_consts::process::RUSAGE_CHILDREN => {}
        _ => return Err(PosixErrno::Invalid),
    }

    let tick = crate::kernel::watchdog::global_tick();

    #[cfg(feature = "process_abstraction")]
    let (ru_minflt, ru_majflt) = {
        let pid = getpid();
        if pid == 0 {
            (0u64, 0u64)
        } else if let Some((_regions, pages)) =
            crate::kernel::launch::process_mapping_state(crate::interfaces::task::ProcessId(pid))
        {
            let p = pages as u64;
            (p, p / 8)
        } else {
            (0u64, 0u64)
        }
    };

    #[cfg(not(feature = "process_abstraction"))]
    let (ru_minflt, ru_majflt) = (0u64, 0u64);

    Ok(PosixRusage {
        ru_utime_ticks: tick,
        ru_stime_ticks: 0,
        ru_maxrss: ru_minflt.saturating_mul(4096),
        ru_minflt,
        ru_majflt,
        ru_nswap: 0,
    })
}

pub(super) fn getpgid_of(pid: usize) -> Result<usize, PosixErrno> {
    if pid == 0 || !process_exists(pid) {
        return Err(PosixErrno::NoEntry);
    }
    ensure_process_metadata(pid);
    Ok(PROCESS_GROUPS.lock().get(&pid).copied().unwrap_or(pid))
}

pub(super) fn parent_of(pid: usize) -> Result<usize, PosixErrno> {
    if pid == 0 || !process_exists(pid) {
        return Err(PosixErrno::NoEntry);
    }
    ensure_process_metadata(pid);
    Ok(PROCESS_PARENTS.lock().get(&pid).copied().unwrap_or(0))
}

pub(super) fn pidfd_open(pid: usize) -> Result<u32, PosixErrno> {
    if !process_exists(pid) {
        return Err(PosixErrno::NoEntry);
    }

    let fd = NEXT_PIDFD.fetch_add(1, Ordering::Relaxed);
    PIDFD_TABLE.lock().insert(fd, pid);
    Ok(fd)
}

pub(super) fn pidfd_get_pid(pidfd: u32) -> Result<usize, PosixErrno> {
    PIDFD_TABLE
        .lock()
        .get(&pidfd)
        .copied()
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub(super) fn pidfd_send_signal(pidfd: u32, signal: i32) -> Result<(), PosixErrno> {
    let pid = pidfd_get_pid(pidfd)?;
    kill(pid, signal)
}

pub(super) fn pidfd_close(pidfd: u32) -> Result<(), PosixErrno> {
    PIDFD_TABLE
        .lock()
        .remove(&pidfd)
        .map(|_| ())
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub(super) fn alarm(seconds: usize) -> usize {
    let pid = getpid();
    if pid == 0 {
        return 0;
    }

    let now_ticks = crate::kernel::watchdog::global_tick();
    let hz = 1_000_000_000 / crate::config::KernelConfig::time_slice();

    let mut table = ALARM_TABLE.lock();
    let old_expire = table.remove(&pid).unwrap_or(0);

    if seconds > 0 {
        let expire = now_ticks.saturating_add(alarm_ticks_from_seconds(seconds, hz));
        table.insert(pid, expire);
    }

    remaining_alarm_seconds(old_expire, now_ticks, hz)
}

pub(super) fn get_process_name(pid: usize) -> Result<alloc::string::String, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        if let Some(proc) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        {
            let name_guard = proc.name.lock();
            let mut name_end = name_guard.len();
            while name_end > 0 && name_guard[name_end - 1] == 0 {
                name_end -= 1;
            }
            return Ok(alloc::string::String::from_utf8_lossy(&name_guard[..name_end]).into_owned());
        }
        Err(PosixErrno::NoEntry)
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = pid;
        Ok(alloc::string::String::from("aethercore"))
    }
}

pub(super) fn set_process_name(pid: usize, name: &str) -> Result<(), PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        if let Some(proc) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        {
            proc.rename(name.as_bytes());
            return Ok(());
        }
        Err(PosixErrno::NoEntry)
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (pid, name);
        Ok(())
    }
}
