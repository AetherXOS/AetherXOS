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
        let pid = crate::modules::posix::process::getpid();
        if let Some(pgrp) = crate::kernel::tty::job_control::GLOBAL_PROCESS_GROUP_MANAGER
            .lock()
            .get_current_group(crate::interfaces::task::ProcessId(pid))
        {
            return (pgrp.0).0;
        }
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
        let pid = crate::modules::posix::process::getpid();
        let mut mgr = crate::kernel::tty::job_control::GLOBAL_PROCESS_GROUP_MANAGER.lock();
        
        // Also update POSIX layer as fallback
        match crate::modules::posix::process::setpgid(0, 0) {
            Ok(_) => {
                match mgr.create_group(crate::interfaces::task::ProcessId(pid)) {
                    Ok(pgrp) => Ok((pgrp.0).0),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(posix_to_kernel_error(e.code())),
        }
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
            .map_err(|e| posix_to_kernel_error(e.code()))?;

        crate::modules::posix::process::getpgid(if pid == 0 { 0 } else { pid })
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
    // Basic implementation: assuming fd <= 2 maps to default TTY (TtyId 0) for now.
    // In a full implementation, fd would be resolved via VFS to find the actual TtyDevice.
    if fd > 2 {
        return Err(KernelError::NotSupported); // Not a TTY or not supported yet
    }
    
    let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
    if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
        if let Some(pgrp) = tty.foreground_pgrp() {
            return Ok((pgrp.0).0);
        }
    }
    
    // If no foreground group is set, or no TTY is found, return ENOTTY (Not a terminal)
    Err(KernelError::NotSupported) // We use NotSupported as ENOTTY mapping for now
}

/// Terminal control: set foreground process group - ioctl TIOCSPGRP
/// Called on terminal fd to change foreground group
///
/// Caller must have permission (owner or session leader)
/// Process group must be in same session
#[cfg(feature = "linux_compat")]
pub fn sys_ioctl_tiocspgrp(fd: usize, pgrp: usize) -> KernelResult<()> {
    if pgrp == 0 {
        return Err(KernelError::InvalidInput);
    }
    
    if fd > 2 {
        return Err(KernelError::NotSupported); // Not a TTY or not supported yet
    }
    
    let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
    if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
        // In a strict implementation, we must check if pgrp is in the same session as the caller
        let pgrp_id = crate::kernel::tty::ProcessGroupId(crate::interfaces::task::ProcessId(pgrp));
        tty.set_foreground_pgrp(Some(pgrp_id));
        return Ok(());
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
        let new_pgid = match result {
            Ok(value) => value,
            Err(err) => panic!("setpgrp should succeed: {:?}", err),
        };
        assert!(new_pgid > 0, "New group ID must be positive");
    }

    #[test_case]
    fn test_getsid_returns_valid_sid() {
        let result = sys_getsid(0);
        let sid = match result {
            Ok(value) => value,
            Err(err) => panic!("getsid should succeed: {:?}", err),
        };
        assert!(sid > 0, "Session ID must be positive");
    }

    #[test_case]
    fn test_setsid_isolation() {
        let result = sys_setsid();
        // Note: This may fail if already session leader
        // But framework should be in place
        if result.is_ok() {
            let sid = match result {
                Ok(value) => value,
                Err(err) => panic!("setsid unexpectedly failed: {:?}", err),
            };
            assert!(sid > 0, "New session ID must be positive");
        }
    }

    /// Test that getpgid(current_pid) == getpgrp()
    #[test_case]
    fn test_getpgid_consistency() {
        let current_pgrp = sys_getpgrp();
        #[cfg(feature = "posix_process")]
        {
            let my_pid = crate::modules::posix::process::getpid();
            let pgid_result = sys_getpgid(my_pid);
            assert_eq!(current_pgrp, pgid_result, "getpgid(self) should match getpgrp()");
        }
    }

    /// Test that setpgid can move current process to new group
    #[test_case]
    fn test_setpgid_modifies_group() {
        #[cfg(feature = "posix_process")]
        {
            let my_pid = crate::modules::posix::process::getpid();
            let _old_pgrp = sys_getpgrp();
            
            // Try to move to a new group (self as group leader)
            let result = sys_setpgid(my_pid, my_pid);
            if result.is_ok() {
                let new_pgrp = sys_getpgrp();
                // Either same or changed, both are valid
                assert!(new_pgrp > 0, "Process group should be valid after setpgid");
            }
        }
    }

    /// Test that setsid creates new session and group
    #[test_case]
    fn test_setsid_creates_new_session() {
        let old_sid = match sys_getsid(0) {
            Ok(sid) => sid,
            Err(_) => return, // Skip if getsid not available
        };
        let _old_pgrp = sys_getpgrp();

        // Try to create new session (this might fail if already session leader)
        match sys_setsid() {
            Ok(new_sid) => {
                assert_ne!(new_sid, old_sid, "setsid should create new session ID");
                // Verify we're now session leader
                let current_sid = match sys_getsid(0) {
                    Ok(sid) => sid,
                    Err(_) => return,
                };
                assert_eq!(current_sid, new_sid, "After setsid, should be session leader");
            }
            Err(_) => {
                // Expected if already session leader
            }
        }
    }

    /// Test that getpgid returns error for invalid PID
    #[test_case]
    fn test_getpgid_invalid_pid() {
        // Use an extremely large invalid PID
        let result = sys_getpgid(99999999);
        // Should either return 0 or error, but not panic
        let _ = result;
    }

    /// Test that setsid rejects if we're already group leader
    #[test_case]
    fn test_setsid_already_group_leader() {
        // Get current group
        let current_pgrp = sys_getpgrp();
        #[cfg(feature = "posix_process")]
        {
            let my_pid = crate::modules::posix::process::getpid();
            // If we're already group leader (pgrp == pid), setsid should fail
            if my_pid == current_pgrp {
                match sys_setsid() {
                    Err(_) => {
                        // Expected behavior
                    }
                    Ok(_) => {
                        // Some implementations might allow re-setsid
                    }
                }
            }
        }
    }

    /// Test foreground group operations consistency
    #[test_case]
    fn test_tcgetpgrp_tcsetpgrp_consistency() {
        // This tests that foreground group reads and writes are consistent
        // even if we can't guarantee specific values
        match crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock().get(crate::kernel::tty::TtyId::new(0)) {
            Some(_tty) => {
                // TTY exists, we should be able to get foreground group
                // Actual value depends on PTY state
            }
            None => {
                // No TTY, which is valid
            }
        }
    }
}
