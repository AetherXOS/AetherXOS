//! High-performance O(1) Syscall Dispatch Table for Linux compatibility.
//!
//! Instead of linear dispatching through multiple match statements, this module
//! uses a static lookup table to route syscalls directly to their handlers.

use super::{SyscallDispFrame, SyscallFrame};

/// Type definition for a syscall handler function.
pub type SyscallHandler = fn(
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> usize;

/// Maximum number of syscalls supported in the table.
pub const MAX_SYSCALLS: usize = 512;

/// The global syscall dispatch table.
/// Initialized with a default 'no_sys' handler.
pub static mut SYSCALL_TABLE: [Option<SyscallHandler>; MAX_SYSCALLS] = [None; MAX_SYSCALLS];

/// Register a syscall handler in the table.
pub fn register_syscall(nr: usize, handler: SyscallHandler) {
    if nr < MAX_SYSCALLS {
        unsafe {
            SYSCALL_TABLE[nr] = Some(handler);
        }
    }
}

/// Fast-path O(1) syscall dispatching.
#[inline(always)]
pub fn dispatch_table(
    nr: usize,
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> Option<usize> {
    if nr < MAX_SYSCALLS {
        unsafe {
            if let Some(handler) = SYSCALL_TABLE[nr] {
                return Some(handler(f, frame));
            }
        }
    }
    None
}
