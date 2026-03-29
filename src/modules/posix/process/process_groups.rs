use super::*;

#[inline(always)]
pub(super) fn getppid() -> usize {
    let pid = getpid();
    if pid == 0 {
        return 0;
    }
    PROCESS_PARENTS.lock().get(&pid).copied().unwrap_or(0)
}

pub(super) fn getpgid(pid: usize) -> Result<usize, PosixErrno> {
    let target = if pid == 0 { getpid() } else { pid };
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }
    ensure_process_metadata(target);
    Ok(PROCESS_GROUPS
        .lock()
        .get(&target)
        .copied()
        .unwrap_or(target))
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

pub(super) fn setsid() -> Result<usize, PosixErrno> {
    let pid = getpid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    ensure_process_metadata(pid);
    let current_pgid = PROCESS_GROUPS.lock().get(&pid).copied().unwrap_or(pid);
    if current_pgid == pid {
        return Err(PosixErrno::PermissionDenied);
    }

    PROCESS_SESSIONS.lock().insert(pid, pid);
    PROCESS_GROUPS.lock().insert(pid, pid);
    Ok(pid)
}

pub(super) fn setpgid(pid: usize, pgid: usize) -> Result<(), PosixErrno> {
    let me = getpid();
    let target = if pid == 0 { me } else { pid };
    if target == 0 || !process_exists(target) {
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
