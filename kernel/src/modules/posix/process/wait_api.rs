use super::*;

pub(super) fn waitpid(pid: usize, nohang: bool) -> Result<Option<usize>, PosixErrno> {
    if let Some(_status) = take_exit_status(pid) {
        return Ok(Some(pid));
    }

    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    if nohang {
        return if process_exists(pid) {
            Ok(None)
        } else {
            Ok(Some(pid))
        };
    }

    loop {
        if !process_exists(pid) {
            return Ok(Some(pid));
        }
        if let Some(_status) = take_exit_status(pid) {
            return Ok(Some(pid));
        }
        let snapshot = current_process_event_epoch();
        if !wait_for_process_event(snapshot) {
            return Err(PosixErrno::TimedOut);
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

pub(super) fn waitpid_status(pid: usize, nohang: bool) -> Result<Option<i32>, PosixErrno> {
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    if let Some(status) = take_exit_status(pid) {
        return Ok(Some(status));
    }

    if nohang {
        return Ok(None);
    }

    loop {
        if let Some(status) = take_exit_status(pid) {
            return Ok(Some(status));
        }
        if !process_exists(pid) {
            return Ok(Some(encode_wait_exit_status(0)));
        }
        let snapshot = current_process_event_epoch();
        if !wait_for_process_event(snapshot) {
            return Err(PosixErrno::TimedOut);
        }
    }
}

pub(super) fn waitpid_status_options(pid: usize, options: i32) -> Result<Option<i32>, PosixErrno> {
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

pub(super) fn wait_status(nohang: bool) -> Result<Option<(usize, i32)>, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        if written == 0 {
            return Ok(None);
        }
        let pid = ids[0].0;
        Ok(waitpid_status(pid, nohang)?.map(|status| (pid, status)))
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let pid = getpid();
        if pid == 0 {
            return Ok(None);
        }
        Ok(waitpid_status(pid, nohang)?.map(|status| (pid, status)))
    }
}

pub(super) fn wait_any_status(nohang: bool) -> Result<Option<(usize, i32)>, PosixErrno> {
    wait_any_status_internal(nohang, true)
}

fn wait_any_status_internal(nohang: bool, consume: bool) -> Result<Option<(usize, i32)>, PosixErrno> {
    if consume {
        if let Some((pid, status)) = EXIT_STATUS_TABLE.lock().pop_first() {
            return Ok(Some((pid, status)));
        }
    } else if let Some((pid, status)) = EXIT_STATUS_TABLE
        .lock()
        .iter()
        .next()
        .map(|(p, s)| (*p, *s))
    {
        return Ok(Some((pid, status)));
    }

    if nohang {
        return Ok(None);
    }

    loop {
        if consume {
            if let Some((pid, status)) = EXIT_STATUS_TABLE.lock().pop_first() {
                return Ok(Some((pid, status)));
            }
        } else if let Some((pid, status)) = EXIT_STATUS_TABLE
            .lock()
            .iter()
            .next()
            .map(|(p, s)| (*p, *s))
        {
            return Ok(Some((pid, status)));
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
                        if let Some(status) = peek_exit_status(id) {
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
                    Some(encode_wait_exit_status(0))
                }
            } else {
                waitpid_status(id, nohang)?
            };
            status.map(|st| (id, st))
        }
        crate::modules::posix_consts::process::P_PGID => {
            if id != 0 && id != getpgrp() {
                return Err(PosixErrno::NoEntry);
            }
            wait_any_status_internal(nohang, !no_wait)?
        }
        crate::modules::posix_consts::process::P_ALL => wait_any_status_internal(nohang, !no_wait)?,
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
    let status = waitpid_status_options(pid, options)?;
    match status {
        Some(st) => {
            let usage = getrusage(crate::modules::posix_consts::process::RUSAGE_CHILDREN)?;
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
        Some((pid, status)) => {
            let usage = getrusage(crate::modules::posix_consts::process::RUSAGE_CHILDREN)?;
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
    peek_exit_status(pid)
}
