use super::*;

pub(super) fn waitpid(pid: usize, nohang: bool) -> Result<Option<usize>, PosixErrno> {
    // Linux/POSIX spec:
    // pid < -1 : Wait for any child in group -pid
    // pid == -1: Wait for any child (Standard wait)
    // pid == 0 : Wait for any child in current group
    // pid > 0  : Wait for specific child

    if (pid as isize) < -1 {
        // Wait for any child in group -pid
        let group_id = (-(pid as isize)) as usize;
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        let mut found_child = false;
        for i in 0..written {
            let child_pid = ids[i].0;
            if let Some(_process) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(child_pid)) {
                // Check if process belongs to the specified group
                ensure_process_metadata(child_pid);
                let pgid = PROCESS_GROUPS.lock().get(&child_pid).copied().unwrap_or(child_pid);
                if pgid == group_id {
                    if let Some((_status, _rusage)) = take_exit_status(child_pid) {
                        clear_process_metadata(child_pid);
                        return Ok(Some(child_pid));
                    }
                    found_child = true;
                }
            }
        }
        if found_child {
            return wait_any_status(nohang).map(|opt| opt.map(|(p, _, _)| p));
        }
        return Err(PosixErrno::NoEntry);
    }

    if (pid as isize) == -1 || pid == 0 {
        return wait_any_status(nohang).map(|opt| opt.map(|(p, _, _)| p));
    }

    loop {
        if let Some((_status, _rusage)) = take_exit_status(pid) {
            clear_process_metadata(pid);
            return Ok(Some(pid));
        }

        let process_opt = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid));
        
        match process_opt {
            Some(process) => {
                if nohang {
                    return Ok(None);
                }
                // Efficiently sleep until the process exits
                process.exit_wait_queue.wait();
            }
            None => {
                // Not in registry and no exit status -> either not a child or already reaped
                // But we should double check if it actually exists as a process at all
                if process_exists(pid) {
                    // It exists but it's not "exited" yet, and not in our registry?
                    // This might be a child we haven't registered correctly or similar.
                    if nohang { return Ok(None); }
                    let snapshot = current_process_event_epoch();
                    wait_for_process_event(snapshot);
                } else {
                    return Err(PosixErrno::NoEntry);
                }
            }
        }
    }
}


pub(super) fn waitpid_options(pid: usize, options: i32) -> Result<Option<usize>, PosixErrno> {
    let allowed = crate::modules::posix_consts::process::WNOHANG
        | crate::modules::posix_consts::process::WUNTRACED
        | crate::modules::posix_consts::process::WCONTINUED;
    if (options & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }
    waitpid(
        pid,
        (options & crate::modules::posix_consts::process::WNOHANG) != 0,
    )
}

pub(super) fn waitpid_status(pid: usize, nohang: bool) -> Result<Option<(i32, PosixRusage)>, PosixErrno> {
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    if let Some(res) = take_exit_status(pid) {
        return Ok(Some(res));
    }

    if nohang {
        return Ok(None);
    }

    loop {
        if let Some(res) = take_exit_status(pid) {
            return Ok(Some(res));
        }
        if !process_exists(pid) {
            return Ok(Some((encode_wait_exit_status(0), PosixRusage::default())));
        }
        let snapshot = current_process_event_epoch();
        if !wait_for_process_event(snapshot) {
            return Err(PosixErrno::TimedOut);
        }
    }
}

pub(super) fn waitpid_status_options(pid: usize, options: i32) -> Result<Option<(i32, PosixRusage)>, PosixErrno> {
    let allowed = crate::modules::posix_consts::process::WNOHANG
        | crate::modules::posix_consts::process::WUNTRACED
        | crate::modules::posix_consts::process::WCONTINUED;
    if (options & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }
    waitpid_status(
        pid,
        (options & crate::modules::posix_consts::process::WNOHANG) != 0,
    )
}

pub(super) fn wait(nohang: bool) -> Result<Option<usize>, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        if written == 0 {
            return Ok(None);
        }
        waitpid(ids[0].0, nohang)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let pid = getpid();
        if pid == 0 {
            Ok(None)
        } else {
            waitpid(pid, nohang)
        }
    }
}

pub(super) fn wait_status(nohang: bool) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        if written == 0 {
            return Ok(None);
        }
        let pid = ids[0].0;
        Ok(waitpid_status(pid, nohang)?.map(|(status, rusage)| (pid, status, rusage)))
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let pid = getpid();
        if pid == 0 {
            return Ok(None);
        }
        Ok(waitpid_status(pid, nohang)?.map(|(status, rusage)| (pid, status, rusage)))
    }
}

pub(super) fn wait_any_status(nohang: bool) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    wait_any_status_internal(nohang, true)
}

fn wait_any_status_internal(nohang: bool, consume: bool) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    if consume {
        if let Some((pid, (status, rusage))) = EXIT_STATUS_TABLE.lock().pop_first() {
            return Ok(Some((pid, status, rusage)));
        }
    } else if let Some((pid, (status, rusage))) = EXIT_STATUS_TABLE
        .lock()
        .iter()
        .next()
        .map(|(p, s)| (*p, *s))
    {
        return Ok(Some((pid, status, rusage)));
    }

    if nohang {
        return Ok(None);
    }

    loop {
        if consume {
            if let Some((pid, (status, rusage))) = EXIT_STATUS_TABLE.lock().pop_first() {
                return Ok(Some((pid, status, rusage)));
            }
        } else if let Some((pid, (status, rusage))) = EXIT_STATUS_TABLE
            .lock()
            .iter()
            .next()
            .map(|(p, s)| (*p, *s))
        {
            return Ok(Some((pid, status, rusage)));
        }
        let snapshot = current_process_event_epoch();
        if !wait_for_process_event(snapshot) {
            return Err(PosixErrno::TimedOut);
        }
    }
}

pub(super) fn waitid(
    id_type: i32,
    id: usize,
    options: i32,
) -> Result<Option<PosixWaitIdInfo>, PosixErrno> {
    let allowed = crate::modules::posix_consts::process::WNOHANG
        | crate::modules::posix_consts::process::WUNTRACED
        | crate::modules::posix_consts::process::WCONTINUED
        | crate::modules::posix_consts::process::WEXITED
        | crate::modules::posix_consts::process::WNOWAIT;
    if (options & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }
    let nohang = (options & crate::modules::posix_consts::process::WNOHANG) != 0;
    let no_wait = (options & crate::modules::posix_consts::process::WNOWAIT) != 0;

    let waited = match id_type {
        crate::modules::posix_consts::process::P_PID => {
            if id == 0 {
                return Err(PosixErrno::Invalid);
            }
            let status = if no_wait {
                if let Some(status) = peek_exit_status(id) {
                    Some(status)
                } else if nohang {
                    None
                } else if process_exists(id) {
                    loop {
                        if let Some((status, _rusage)) = peek_exit_status(id) {
                            return Ok(Some(PosixWaitIdInfo {
                                pid: id,
                                status,
                                code: wait_code_from_status(status),
                            }));
                        }
                        let snapshot = current_process_event_epoch();
                        if !wait_for_process_event(snapshot) {
                            return Err(PosixErrno::TimedOut);
                        }
                    }
                } else {
                    Some((encode_wait_exit_status(0), PosixRusage::default()))
                }
            } else {
                waitpid_status(id, nohang)?
            };
            status.map(|(st, _ru)| (id, st))
        }
        crate::modules::posix_consts::process::P_PGID => {
            if id != 0 && id != getpgrp() {
                return Err(PosixErrno::NoEntry);
            }
            wait_any_status_internal(nohang, !no_wait)?.map(|(p, s, _ru)| (p, s))
        }
        crate::modules::posix_consts::process::P_ALL => {
            wait_any_status_internal(nohang, !no_wait)?.map(|(p, s, _ru)| (p, s))
        }
        _ => return Err(PosixErrno::Invalid),
    };

    Ok(waited.map(|(pid, status)| PosixWaitIdInfo {
        pid,
        status,
        code: wait_code_from_status(status),
    }))
}

pub(super) fn wait4(
    pid: usize,
    options: i32,
) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    let res = waitpid_status_options(pid, options)?;
    match res {
        Some((st, usage)) => {
            Ok(Some((pid, st, usage)))
        }
        None => Ok(None),
    }
}

pub(super) fn wait3(options: i32) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    let allowed = crate::modules::posix_consts::process::WNOHANG
        | crate::modules::posix_consts::process::WUNTRACED
        | crate::modules::posix_consts::process::WCONTINUED;
    if (options & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }

    let nohang = (options & crate::modules::posix_consts::process::WNOHANG) != 0;
    match wait_any_status(nohang)? {
        Some((pid, status, usage)) => {
            Ok(Some((pid, status, usage)))
        }
        None => Ok(None),
    }
}

#[inline(always)]
pub(super) fn pending_exit_status_count() -> usize {
    EXIT_STATUS_TABLE.lock().len()
}

#[inline(always)]
pub(super) fn get_cached_exit_status(pid: usize) -> Option<i32> {
    peek_exit_status(pid).map(|(s, _)| s)
}
