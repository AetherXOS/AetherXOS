//! High-performance O(1) Syscall Dispatch Table for Linux compatibility.
//!
//! Instead of linear dispatching through multiple match statements, this module
//! uses a static lookup table to route syscalls directly to their handlers.

use crate::kernel::sync::IrqSafeMutex;

use super::{SyscallDispFrame, SyscallFrame};

/// Type definition for a syscall handler function.
pub type SyscallHandler = fn(
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> usize;

/// Maximum number of syscalls supported in the table.
pub const MAX_SYSCALLS: usize = 512;

/// The global syscall dispatch table, protected by an IRQ-safe mutex.
/// This prevents data races in multi-core / interrupt contexts.
static SYSCALL_TABLE: IrqSafeMutex<[Option<SyscallHandler>; MAX_SYSCALLS]> =
    IrqSafeMutex::new([None; MAX_SYSCALLS]);

/// Register a syscall handler in the table.
pub fn register_syscall(nr: usize, handler: SyscallHandler) {
    if nr < MAX_SYSCALLS {
        SYSCALL_TABLE.lock()[nr] = Some(handler);
    }
}

/// Fast-path O(1) syscall dispatching.
/// Acquires the table lock only to copy the handler pointer,
/// then releases it before invoking the handler to minimize contention.
#[inline(always)]
pub fn dispatch_table(
    nr: usize,
    f: &mut SyscallDispFrame,
    frame: &mut SyscallFrame,
) -> Option<usize> {
    if nr >= MAX_SYSCALLS {
        return None;
    }
    let handler = {
        let table = SYSCALL_TABLE.lock();
        table[nr]
    };
    handler.map(|h| h(f, frame))
}
