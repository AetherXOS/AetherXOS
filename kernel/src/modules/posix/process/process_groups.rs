use super::*;

#[inline(always)]
pub(super) fn getppid() -> usize {
    getppid_of(getpid()).unwrap_or(0)
}

pub(super) fn getppid_of(pid: usize) -> Result<usize, PosixErrno> {
    if pid == 0 || !process_exists(pid) {
        return Err(PosixErrno::NoEntry);
    }
    Ok(PROCESS_PARENTS.lock().get(&pid).copied().unwrap_or(0))
}

pub(super) fn getpgid(pid: usize) -> Result<usize, PosixErrno> {
    let target = if pid == 0 { getpid() } else { pid };
    if target == 0 {
        return Err(PosixErrno::NoEntry);
    }

    #[cfg(feature = "process_abstraction")]
    {
        if let Some(proc) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(target)) {
            let pgid = proc.pgid.load(Ordering::Relaxed);
            return Ok(if pgid == 0 { target } else { pgid as usize });
        }
        return Err(PosixErrno::NoEntry);
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if !process_exists(target) {
            return Err(PosixErrno::NoEntry);
        }
        ensure_process_metadata(target);
        Ok(PROCESS_GROUPS
            .lock()
            .get(&target)
            .copied()
            .unwrap_or(target))
    }
}

#[inline(always)]
pub(super) fn getpgrp() -> usize {
    getpgid(0).unwrap_or(0)
}

pub(super) fn getsid(pid: usize) -> Result<usize, PosixErrno> {
    let target = if pid == 0 { getpid() } else { pid };
    if target == 0 {
        return Err(PosixErrno::Invalid);
    }

    #[cfg(feature = "process_abstraction")]
    {
        if let Some(proc) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(target)) {
            let sid = proc.sid.load(Ordering::Relaxed);
            return Ok(if sid == 0 { target } else { sid as usize });
        }
        return Err(PosixErrno::NoEntry);
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if process_exists(target) {
            ensure_process_metadata(target);
            Ok(PROCESS_SESSIONS
                .lock()
                .get(&target)
                .copied()
                .unwrap_or(target))
        } else {
            Err(PosixErrno::NoEntry)
        }
    }
}

pub(super) fn setsid() -> Result<usize, PosixErrno> {
    let pid = getpid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    #[cfg(feature = "process_abstraction")]
    {
        if let Some(proc) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) {
            let current_pgid = proc.pgid.load(Ordering::Relaxed);
            if current_pgid as usize == pid {
                return Err(PosixErrno::PermissionDenied);
            }
            proc.sid.store(pid as u32, Ordering::Relaxed);
            proc.pgid.store(pid as u32, Ordering::Relaxed);
            return Ok(pid);
        }
        return Err(PosixErrno::NoEntry);
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        ensure_process_metadata(pid);
        let current_pgid = PROCESS_GROUPS.lock().get(&pid).copied().unwrap_or(pid);
        if current_pgid == pid {
            return Err(PosixErrno::PermissionDenied);
        }

        PROCESS_SESSIONS.lock().insert(pid, pid);
        PROCESS_GROUPS.lock().insert(pid, pid);
        Ok(pid)
    }
}

pub(super) fn setpgid(pid: usize, pgid: usize) -> Result<(), PosixErrno> {
    let me = getpid();
    let target = if pid == 0 { me } else { pid };
    if target == 0 {
        return Err(PosixErrno::NoEntry);
    }

    #[cfg(feature = "process_abstraction")]
    {
        let proc = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(target))
            .ok_or(PosixErrno::NoEntry)?;
        
        let desired = if pgid == 0 { target } else { pgid };
        
        // POSIX: session must be the same
        let my_sid = proc.sid.load(Ordering::Relaxed);
        if desired != target {
            let group_proc = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(desired))
                .ok_or(PosixErrno::NoEntry)?;
            if group_proc.sid.load(Ordering::Relaxed) != my_sid {
                return Err(PosixErrno::PermissionDenied);
            }
        }
        
        proc.pgid.store(desired as u32, Ordering::Relaxed);
        Ok(())
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if !process_exists(target) {
            return Err(PosixErrno::NoEntry);
        }
        ensure_process_metadata(target);
        let desired = if pgid == 0 { target } else { pgid };
        if desired != target && !process_exists(desired) {
            return Err(PosixErrno::NoEntry);
        }

        let sid_target = PROCESS_SESSIONS
            .lock()
            .get(&target)
            .copied()
            .unwrap_or(target);
        let sid_group = PROCESS_SESSIONS
            .lock()
            .get(&desired)
            .copied()
            .unwrap_or(sid_target);
        if sid_group != sid_target {
            return Err(PosixErrno::PermissionDenied);
        }

        PROCESS_GROUPS.lock().insert(target, desired);
        Ok(())
    }
}

#[cfg(feature = "process_abstraction")]
pub(super) fn process_count() -> usize {
    crate::kernel::launch::process_count()
}

#[cfg(not(feature = "process_abstraction"))]
pub(super) fn process_count() -> usize {
    if gettid() == 0 {
        0
    } else {
        1
    }
}

#[cfg(feature = "process_abstraction")]
pub(super) fn process_ids_snapshot(out: &mut [usize]) -> usize {
    let mut typed = alloc::vec![crate::interfaces::task::ProcessId(0); out.len()];
    let written = crate::kernel::launch::process_ids_snapshot(&mut typed);
    for (dst, src) in out.iter_mut().zip(typed.iter()).take(written) {
        *dst = src.0;
    }
    written
}

#[cfg(not(feature = "process_abstraction"))]
pub(super) fn process_ids_snapshot(out: &mut [usize]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let pid = getpid();
    if pid == 0 {
        return 0;
    }
    out[0] = pid;
    1
}
