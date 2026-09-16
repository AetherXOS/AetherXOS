//! Syscall security verification layer.
//!
//! Provides centralized security checks for syscall dispatch.

use crate::interfaces::task::TaskId;
use crate::interfaces::security::SecurityContext;

// ---------------------------------------------------------------------------
// Error Type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    NullPointer,
    MisalignedAccess,
    NotUserMemory,
    InsufficientCapabilities,
    SyscallNotAllowed,
    InvalidArgument,
}

impl core::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NullPointer => write!(f, "null pointer"),
            Self::MisalignedAccess => write!(f, "misaligned access"),
            Self::NotUserMemory => write!(f, "not user memory"),
            Self::InsufficientCapabilities => write!(f, "insufficient capabilities"),
            Self::SyscallNotAllowed => write!(f, "syscall not allowed"),
            Self::InvalidArgument => write!(f, "invalid argument"),
        }
    }
}
// ---------------------------------------------------------------------------
// Security Guard (RAII)
// ---------------------------------------------------------------------------

pub struct SyscallSecurityGuard {
    syscall_id: usize,
    start_tick: u64,
}

impl SyscallSecurityGuard {
    pub fn new(syscall_id: usize) -> Self {
        let start_tick = crate::kernel::watchdog::global_tick();
        Self { syscall_id, start_tick }
    }

    pub fn syscall_id(&self) -> usize { self.syscall_id }

    pub fn record_result(&self, _result: Result<usize, SecurityError>) {
        #[cfg(feature = "debug_observability_all")]
        match _result {
            Ok(ret) => crate::klog_trace!("[SECURITY] syscall {} OK ret={:#x}", self.syscall_id, ret),
            Err(e) => crate::klog_warn!("[SECURITY] syscall {} DENIED: {}", self.syscall_id, e),
        }
    }
}

impl Drop for SyscallSecurityGuard {
    fn drop(&mut self) {
        #[cfg(feature = "debug_observability_all")]
        crate::klog_trace!("[SECURITY] syscall {} exit", self.syscall_id);
    }
}
// ---------------------------------------------------------------------------
// Memory Validation
// ---------------------------------------------------------------------------

pub fn validate_user_ptr(ptr: usize, size: usize) -> Result<(), SecurityError> {
    if ptr == 0 { return Err(SecurityError::NullPointer); }
    let end = ptr.checked_add(size).ok_or(SecurityError::InvalidArgument)?;
    if end <= ptr && size > 0 { return Err(SecurityError::InvalidArgument); }
    if ptr >= 0xFFFF8000_0000_0000 { return Err(SecurityError::NotUserMemory); }
    Ok(())
}

pub fn validate_user_ptr_aligned(ptr: usize, alignment: usize) -> Result<(), SecurityError> {
    if ptr == 0 { return Err(SecurityError::NullPointer); }
    if alignment == 0 || ptr % alignment != 0 { return Err(SecurityError::MisalignedAccess); }
    Ok(())
}

pub fn check_capability(_tid: TaskId, _cap: u64, _ctx: &SecurityContext) -> Result<(), SecurityError> {
    if _ctx.is_kernel { return Ok(()); }
    Ok(())
}

pub fn validate_user_string_ptr(ptr: usize, max_len: usize) -> Result<(usize, usize), SecurityError> {
    validate_user_ptr(ptr, 1)?;
    let safe_max = max_len.min((0xFFFF8000_0000_0000u64).saturating_sub(ptr as u64) as usize);
    Ok((ptr, safe_max))
}

pub fn validate_user_array<T>(ptr: usize, count: usize) -> Result<(), SecurityError> {
    let size = core::mem::size_of::<T>().checked_mul(count).ok_or(SecurityError::InvalidArgument)?;
    validate_user_ptr(ptr, size)?;
    if ptr % core::mem::align_of::<T>() != 0 { return Err(SecurityError::MisalignedAccess); }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_validate_null_ptr() {
        assert_eq!(validate_user_ptr(0, 4), Err(SecurityError::NullPointer));
    }

    #[test_case]
    fn test_validate_kernel_ptr() {
        assert_eq!(validate_user_ptr(0xFFFF8000_0000_1000, 4), Err(SecurityError::NotUserMemory));
    }

    #[test_case]
    fn test_validate_user_ptr_valid() {
        assert!(validate_user_ptr(0x7F00_0000_1000, 4).is_ok());
    }

    #[test_case]
    fn test_validate_ptr_wraparound() {
        assert_eq!(validate_user_ptr(usize::MAX - 3, 8), Err(SecurityError::InvalidArgument));
    }

    #[test_case]
    fn test_validate_alignment() {
        assert_eq!(validate_user_ptr_aligned(0x7F00_0000_1001, 4), Err(SecurityError::MisalignedAccess));
        assert!(validate_user_ptr_aligned(0x7F00_0000_1000, 4).is_ok());
    }

    #[test_case]
    fn test_validate_user_array() {
        assert!(validate_user_array::<u64>(0x7F00_0000_1000, 10).is_ok());
        assert_eq!(validate_user_array::<u64>(0x7F00_0000_1001, 10), Err(SecurityError::MisalignedAccess));
    }
}
