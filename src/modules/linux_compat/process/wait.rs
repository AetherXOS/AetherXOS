use super::super::*;
use crate::modules::posix_consts::process::{RUSAGE_CHILDREN, WNOHANG};

#[inline]
fn zero_rusage() -> crate::modules::posix::process::PosixRusage {
    crate::modules::posix::process::PosixRusage {
        ru_utime_ticks: 0,
        ru_stime_ticks: 0,
        ru_maxrss: 0,
        ru_minflt: 0,
        ru_majflt: 0,
        ru_nswap: 0,
    }
}

/// `wait4(2)` — Wait for status changes in a child of the calling process, legacy version.
pub fn sys_linux_wait4(
    upid: isize,
    stat_addr: UserPtr<i32>,
    options: usize,
    ru_addr: UserPtr<LinuxRusage>,
) -> usize {
    crate::require_posix_process!((upid, stat_addr, options, ru_addr) => {
        let zero_rusage = zero_rusage();
        let opts = options as i32;
        let nohang = (opts & WNOHANG) != 0;

        let res = if upid == -1 {
            match crate::modules::posix::process::wait_any_status(nohang) {
                Ok(Some((p, st))) => {
                    let rusage = crate::modules::posix::process::getrusage(RUSAGE_CHILDREN as i32).unwrap_or(zero_rusage);
                    Ok(Some((p, st, rusage)))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        } else if upid == 0 {
            // waitpid(pid=0): wait for any child in the caller's process group.
            let opts_waitid = opts | crate::modules::posix_consts::process::WEXITED;
            match crate::modules::posix::process::waitid(crate::modules::posix_consts::process::P_PGID, 0, opts_waitid) {
                Ok(Some(info)) => {
                    let rusage = crate::modules::posix::process::getrusage(RUSAGE_CHILDREN as i32).unwrap_or(zero_rusage);
                    Ok(Some((info.pid, info.status, rusage)))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        } else if upid > 0 {
            // Specific PID wait
            crate::modules::posix::process::wait4(upid as usize, opts)
        } else {
            // waitpid(pid<-1): wait for any child in process group |pid|.
            let pgid = (-upid) as usize;
            let opts_waitid = opts | crate::modules::posix_consts::process::WEXITED;
            match crate::modules::posix::process::waitid(crate::modules::posix_consts::process::P_PGID, pgid, opts_waitid) {
                Ok(Some(info)) => {
                    let rusage = crate::modules::posix::process::getrusage(RUSAGE_CHILDREN as i32).unwrap_or(zero_rusage);
                    Ok(Some((info.pid, info.status, rusage)))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        };

        match res {
            Ok(Some((wpid, status, rusage))) => {
                if !stat_addr.is_null() {
                    let _ = stat_addr.write(&(status as i32));
                }
                if !ru_addr.is_null() {
                    let _ = ru_addr.write(&fill_linux_rusage(rusage));
                }
                wpid
            }
            Ok(None) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_waitpid(pid: isize, stat_addr: UserPtr<i32>, options: usize) -> usize {
    sys_linux_wait4(pid, stat_addr, options, UserPtr::new(0))
}

/// `waitid(2)` — Wait for status changes in a child of the calling process, modern version.
pub fn sys_linux_waitid(
    idtype: usize,
    id: usize,
    infop: UserPtr<LinuxSiginfo>,
    options: usize,
    ru_addr: UserPtr<LinuxRusage>,
) -> usize {
    crate::require_posix_process!((idtype, id, infop, options, ru_addr) => {
        let zero_rusage = zero_rusage();
        // idtype: P_ALL (0), P_PID (1), P_PGID (2)
        match crate::modules::posix::process::waitid(idtype as i32, id, options as i32) {
            Ok(Some(info)) => {
                if !infop.is_null() {
                    let mut sinfo: LinuxSiginfo = unsafe { core::mem::zeroed() };
                    sinfo.si_signo = linux::SIGCHLD as i32;
                    sinfo.si_code = info.code; // CLD_EXITED, CLD_KILLED, etc.
                    sinfo.si_pid = info.pid as i32;
                    sinfo.si_uid = 0; // Root by default
                    sinfo.si_status = info.status;

                    if let Err(e) = infop.write(&sinfo) { return e; }
                }

                if !ru_addr.is_null() {
                    let rusage = crate::modules::posix::process::getrusage(RUSAGE_CHILDREN as i32).unwrap_or(zero_rusage);
                    let _ = ru_addr.write(&fill_linux_rusage(rusage));
                }
                0
            }
            Ok(None) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}
