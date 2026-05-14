//! Syscall Registration - Populates the O(1) dispatch table with handlers.

use super::table::register_syscall;
use crate::hal::syscalls_consts::linux_nr;
use crate::modules::linux_compat::fs::file::*;
use crate::modules::linux_compat::fs::attr::*;
use crate::modules::linux_compat::fs::dir::*;
use crate::modules::linux_compat::fs::mount::*;
use crate::modules::linux_compat::fs::stat::*;
use crate::modules::linux_compat::memory::mman::*;
use crate::modules::linux_compat::types::{Fd, UserPtr};

/// Populates the global syscall table with standard handlers.
pub fn populate_table() {
    // --- File Operations ---
    register_syscall(linux_nr::READ, |f, _| sys_linux_read(fd!(f.a1), uptr!(f.a2), f.a3));
    register_syscall(linux_nr::WRITE, |f, _| sys_linux_write(fd!(f.a1), uptr!(f.a2), f.a3));
    register_syscall(linux_nr::OPEN, |f, _| sys_linux_open(uptr!(f.a1), f.a2, f.a3));
    register_syscall(linux_nr::CLOSE, |f, _| sys_linux_close(fd!(f.a1)));
    register_syscall(linux_nr::LSEEK, |f, _| sys_linux_lseek(fd!(f.a1), f.a2 as i64, f.a3));
    register_syscall(linux_nr::DUP, |f, _| sys_linux_dup(fd!(f.a1)));
    register_syscall(linux_nr::DUP2, |f, _| sys_linux_dup2(fd!(f.a1), fd!(f.a2)));
    register_syscall(linux_nr::PIPE, |f, _| sys_linux_pipe(uptr!(f.a1)));
    register_syscall(linux_nr::PIPE2, |f, _| sys_linux_pipe2(uptr!(f.a1), f.a2 as i32));

    // --- Directory & FS Management ---
    register_syscall(linux_nr::MKDIR, |f, _| sys_linux_mkdir(uptr!(f.a1), f.a2));
    register_syscall(linux_nr::RMDIR, |f, _| sys_linux_rmdir(uptr!(f.a1)));
    register_syscall(linux_nr::GETDENTS64, |f, _| sys_linux_getdents64(fd!(f.a1), uptr!(f.a2), f.a3));
    register_syscall(linux_nr::CHMOD, |f, _| sys_linux_chmod(uptr!(f.a1), f.a2));
    register_syscall(linux_nr::CHOWN, |f, _| sys_linux_chown(uptr!(f.a1), f.a2, f.a3));

    // --- Process & Memory ---
    register_syscall(linux_nr::EXIT, |f, _| crate::kernel::syscalls::sys_exit(f.a1));
    register_syscall(linux_nr::GETPID, |_, _| crate::kernel::syscalls::sys_getpid());
    register_syscall(linux_nr::BRK, |f, _| sys_linux_brk(f.a1 as u64));
    register_syscall(linux_nr::MMAP, |f, _| sys_linux_mmap(f.a1 as u64, f.a2, f.a3 as i32, f.a4 as i32, f.a5 as i32, f.a6 as i64));
    register_syscall(linux_nr::MUNMAP, |f, _| sys_linux_munmap(f.a1 as u64, f.a2));
    
    // --- File Metadata ---
    register_syscall(linux_nr::FSTAT, |f, _| sys_linux_fstat(fd!(f.a1), uptr!(f.a2)));
    register_syscall(linux_nr::STAT, |f, _| sys_linux_stat(uptr!(f.a1), uptr!(f.a2)));
    register_syscall(linux_nr::LSTAT, |f, _| sys_linux_lstat(uptr!(f.a1), uptr!(f.a2)));
    
    // Add more as needed...
}

// Helpers to make registration closure code cleaner
macro_rules! fd { ($val:expr) => { Fd($val as i32) }; }
macro_rules! uptr { ($val:expr) => { UserPtr::new($val) }; }
