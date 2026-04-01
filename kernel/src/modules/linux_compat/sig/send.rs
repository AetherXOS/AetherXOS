use super::super::*;

pub fn sys_linux_kill(pid: usize, signal: usize) -> usize {
    crate::require_posix_signal!((pid, signal) => {
        match crate::modules::posix::signal::kill(pid, signal as i32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    })
}

pub fn sys_linux_tgkill(tgid: usize, tid: usize, signal: usize) -> usize {
    crate::require_posix_signal!((tgid, tid, signal) => {
        match crate::modules::posix::signal::tgkill(tgid, tid, signal as i32) {
                    Ok(()) => 0,
                    Err(err) => linux_errno(err.code()),
                }
    })
}

pub fn sys_linux_tkill(tid: usize, signal: usize) -> usize {
    crate::require_posix_signal!((tid, signal) => {
        match crate::modules::posix::signal::tkill(tid, signal as i32) {
                    Ok(()) => 0,
                    Err(err) => linux_errno(err.code()),
                }
    })
}
