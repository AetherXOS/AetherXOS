//! Linux Errno Matrix - Process Group & Session Errors
//! 
//! Maps kernel errors to Linux errno values for job control syscalls
//! POSIX compliance for error handling in process group/session management

use crate::interfaces::KernelError;

/// Process group errno codes (Linux ENOSYS = 38, EPERM = 1, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessGroupErrno {
    /// Operation not permitted (e.g., trying to setpgid on someone else's child after exec)
    EPERM = 1,
    
    /// No such process (getpgid/getsid on non-existent pid)
    ESRCH = 3,
    
    /// Bad file descriptor (ioctl on invalid fd)
    EBADF = 9,
    
    /// Invalid argument (pgid out of range, etc.)
    EINVAL = 22,
    
    /// Device not a terminal (TIOCGPGRP/TIOCSPGRP on non-tty)
    ENOTTY = 25,
    
    /// Process group not found (setpgid to non-existent group)
    EPGID_NOT_FOUND = 39, // Linux uses this in some contexts
    
    /// Operation not supported (feature disabled)
    ENOSYS = 38,
}

impl ProcessGroupErrno {
    pub fn as_linux_errno(self) -> i32 {
        self as i32
    }
}

/// Error mapping: kernel internal -> Linux errno for process group operations
pub trait ProcessGroupErrorMapping {
    fn to_errno(&self) -> ProcessGroupErrno;
}

impl ProcessGroupErrorMapping for KernelError {
    fn to_errno(&self) -> ProcessGroupErrno {
        match self {
            KernelError::PermissionDenied => ProcessGroupErrno::EPERM,
            KernelError::NoSuchProcess => ProcessGroupErrno::ESRCH,
            KernelError::InvalidInput => ProcessGroupErrno::EINVAL,
            KernelError::BadDescriptor => ProcessGroupErrno::EBADF,
            KernelError::NotSupported => ProcessGroupErrno::ENOSYS,
            _ => ProcessGroupErrno::EINVAL, // Default to EINVAL for unknown
        }
    }
}

/// Context-specific errno selection for process group operations
pub struct ProcessGroupErrorContext;

impl ProcessGroupErrorContext {
    /// For setpgid: distinguish between permission and not-found
    pub fn setpgid_errno(error: &KernelError) -> i32 {
        match error {
            KernelError::PermissionDenied => 1,    // EPERM
            KernelError::NoSuchProcess => 3,       // ESRCH
            KernelError::InvalidInput => 22,       // EINVAL
            _ => 22,                               // Default EINVAL
        }
    }

    /// For getpgid: primarily ESRCH for not found
    pub fn getpgid_errno(error: &KernelError) -> i32 {
        match error {
            KernelError::NoSuchProcess => 3, // ESRCH
            KernelError::InvalidInput => 22, // EINVAL
            _ => 22,
        }
    }

    /// For setsid: primarily EPERM if already group leader
    pub fn setsid_errno(error: &KernelError) -> i32 {
        match error {
            KernelError::PermissionDenied => 1,    // EPERM
            KernelError::InvalidInput => 22,       // EINVAL
            _ => 22,
        }
    }

    /// For ioctl TIOCGPGRP/TIOCSPGRP: ENOTTY for non-terminals
    pub fn ioctl_errno(error: &KernelError) -> i32 {
        match error {
            KernelError::BadDescriptor => 9,       // EBADF
            KernelError::PermissionDenied => 1,    // EPERM
            KernelError::InvalidInput => 22,       // EINVAL
            _ => 25,                               // Default ENOTTY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_eperm_errno_value() {
        assert_eq!(ProcessGroupErrno::EPERM.as_linux_errno(), 1);
    }

    #[test_case]
    fn test_esrch_errno_value() {
        assert_eq!(ProcessGroupErrno::ESRCH.as_linux_errno(), 3);
    }

    #[test_case]
    fn test_enotty_errno_value() {
        assert_eq!(ProcessGroupErrno::ENOTTY.as_linux_errno(), 25);
    }

    #[test_case]
    fn test_enosys_errno_value() {
        assert_eq!(ProcessGroupErrno::ENOSYS.as_linux_errno(), 38);
    }

    #[test_case]
    fn test_context_getpgid_not_found() {
        let errno = ProcessGroupErrorContext::getpgid_errno(&KernelError::NoSuchProcess);
        assert_eq!(errno, 3, "getpgid should return ESRCH for not found");
    }

    #[test_case]
    fn test_context_setsid_permission() {
        let errno = ProcessGroupErrorContext::setsid_errno(&KernelError::PermissionDenied);
        assert_eq!(errno, 1, "setsid should return EPERM for permission denied");
    }

    #[test_case]
    fn test_context_ioctl_not_terminal() {
        let errno = ProcessGroupErrorContext::ioctl_errno(&KernelError::BadDescriptor);
        assert_eq!(errno, 9, "ioctl should return EBADF for bad descriptor");
    }
}
