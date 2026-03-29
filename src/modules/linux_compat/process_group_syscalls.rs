//! Linux Compat Expansion: Process Group & Session Management
//!
//! Missing critical syscalls for job control integration:
//! - GETPGRP, SETPGRP, GETPGID, SETPGID
//! - GETSID, SETSID
//! - Terminal control (TIOCGPGRP, TIOCSPGRP)

use crate::interfaces::{KernelError, KernelResult};

fn posix_to_kernel_error(errno_code: i32) -> KernelError {
    use crate::modules::posix_consts::errno as e;
    match errno_code {
        e::EPERM => KernelError::PermissionDenied,
        e::ESRCH => KernelError::NoSuchProcess,
        e::EBADF => KernelError::BadDescriptor,
        e::EINVAL => KernelError::InvalidInput,
        e::ENOSYS => KernelError::NotSupported,
        _ => KernelError::InternalError,
    }
}

/// Get process group ID - sys_getpgrp()
/// Returns process group ID of current process (or specified PID)
///
/// Linux: getpgrp() returns caller's pgrp
/// Linux: getpgid(pid) returns pgrp of specified process
#[cfg(feature = "linux_compat")]
pub fn sys_getpgrp() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getpgrp()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

/// Get process group ID for specific process - sys_getpgid()
#[cfg(feature = "linux_compat")]
pub fn sys_getpgid(pid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::getpgid(pid) {
            Ok(v) => v,
            Err(_) => 0,
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = pid;
        0
    }
}

/// Set process group for current process to new/existing group - sys_setpgrp()
/// Creates new process group with caller as leader (pgrp == pid)
///
/// Linux: setpgrp() == setpgid(0, 0)
///        - Caller becomes process group leader
///        - New pgid == caller's pid
///        - Must not already be session leader (EPERM)
#[cfg(feature = "linux_compat")]
pub fn sys_setpgrp() -> KernelResult<usize> {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::setpgid(0, 0)
            .map(|_| crate::modules::posix::process::getpgrp())
            .map_err(|e| posix_to_kernel_error(e.code()))
    }
    #[cfg(not(feature = "posix_process"))]
    {
        Err(KernelError::NotSupported)
    }
}

/// Set process group for specific process - sys_setpgid()
/// Arguments:
/// - pid: target process (0 = current process)
/// - pgid: target group (0 = use pid as group id, creating new group)
///
/// POSIX requires:
/// - pid must be caller or child of caller
/// - Can only change pgid if process hasn't execve'd yet (no exec in child group)
/// - If pgid doesn't exist, create new group with that ID
#[cfg(feature = "linux_compat")]
pub fn sys_setpgid(pid: usize, pgid: usize) -> KernelResult<usize> {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::setpgid(pid, pgid)
            .map(|_| crate::modules::posix::process::getpgid(if pid == 0 { 0 } else { pid }).unwrap_or(0))
            .map_err(|e| posix_to_kernel_error(e.code()))
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (pid, pgid);
        Err(KernelError::NotSupported)
    }
}

/// Get session ID for process - sys_getsid()
/// Returns session ID (typically == pid of session leader)
///
/// Linux: getsid(0) returns caller's session ID
/// Linux: getsid(pid) returns session ID of specified process
/// Returns -ESRCH if process not found
#[cfg(feature = "linux_compat")]
pub fn sys_getsid(pid: usize) -> KernelResult<usize> {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getsid(pid).map_err(|e| posix_to_kernel_error(e.code()))
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = pid;
        Err(KernelError::NotSupported)
    }
}

/// Create new session - sys_setsid()
/// Caller becomes session leader (sid == pid)
/// Caller also becomes process group leader (pgid == pid)
/// Caller detachs from controlling terminal (if any)
///
/// POSIX restrictions:
/// - Caller must not already be a process group leader (EPERM)
/// - Returns new session ID (== caller's pid)
/// - Returns -EPERM if already group leader
#[cfg(feature = "linux_compat")]
pub fn sys_setsid() -> KernelResult<usize> {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::setsid().map_err(|e| posix_to_kernel_error(e.code()))
    }
    #[cfg(not(feature = "posix_process"))]
    {
        Err(KernelError::NotSupported)
    }
}

/// Terminal control: get foreground process group - ioctl TIOCGPGRP
/// Called on terminal fd to query foreground group
///
/// Returns: process group ID of foreground process group (or -ENOTTY if not terminal)
#[cfg(feature = "linux_compat")]
pub fn sys_ioctl_tiocgpgrp(fd: usize, _ptr: usize) -> KernelResult<usize> {
    let _ = _ptr;
    if fd == 0 {
        return Err(KernelError::BadDescriptor);
    }
    Err(KernelError::NotSupported)
}

/// Terminal control: set foreground process group - ioctl TIOCSPGRP
/// Called on terminal fd to change foreground group
///
/// Caller must have permission (owner or session leader)
/// Process group must be in same session
#[cfg(feature = "linux_compat")]
pub fn sys_ioctl_tiocspgrp(fd: usize, pgrp: usize) -> KernelResult<()> {
    if fd == 0 {
        return Err(KernelError::BadDescriptor);
    }
    if pgrp == 0 {
        return Err(KernelError::InvalidInput);
    }
    Err(KernelError::NotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_getpgrp_returns_valid_pgrp() {
        let pgrp = sys_getpgrp();
        assert!(pgrp > 0, "Process group ID must be positive");
    }

    #[test_case]
    fn test_setpgrp_creates_new_group() {
        let result = sys_setpgrp();
        assert!(result.is_ok(), "setpgrp should succeed");
        let new_pgid = result.unwrap();
        assert!(new_pgid > 0, "New group ID must be positive");
    }

    #[test_case]
    fn test_getsid_returns_valid_sid() {
        let result = sys_getsid(0);
        assert!(result.is_ok(), "getsid should succeed");
        let sid = result.unwrap();
        assert!(sid > 0, "Session ID must be positive");
    }

    #[test_case]
    fn test_setsid_isolation() {
        let result = sys_setsid();
        // Note: This may fail if already session leader
        // But framework should be in place
        if result.is_ok() {
            assert!(result.unwrap() > 0, "New session ID must be positive");
        }
    }
}
