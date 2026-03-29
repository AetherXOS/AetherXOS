// Linux syscall compatibility shim (non-linux_compat feature path).
// All `#[cfg(not(feature = "linux_compat"))]` Linux syscall implementations live here.
// In production builds (with `linux_compat` feature), this module is not compiled;
// the full linux_compat layer in `src/modules/linux_compat/` handles dispatch instead.

use super::*;
mod dispatch;
mod fd_process_identity;
mod fs;
mod memory;
mod net;
mod process;
mod signal;
mod task_time;
mod util;
#[cfg(all(test, not(feature = "linux_compat")))]
pub(crate) use process::{execve_stack_required_bytes, prepare_execve_user_stack};
#[cfg(all(test, not(feature = "linux_compat")))]
pub(crate) use util::read_user_c_string_array;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_FUTEX_CMD_MASK: usize = 0x7f;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_MAX_VECTOR_ITEMS: usize = 256;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_STACK_BYTES: u64 = 2 * crate::interfaces::memory::PAGE_SIZE_4K as u64;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_NULL: usize = 0;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_ENTRY: usize = 9;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_PAGESZ: usize = 6;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_BASE: usize = 7;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_FLAGS: usize = 8;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_UID: usize = 11;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_EUID: usize = 12;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_GID: usize = 13;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_EGID: usize = 14;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_SECURE: usize = 23;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_RANDOM: usize = 25;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_HWCAP: usize = 16;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_CLKTCK: usize = 17;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_PLATFORM: usize = 15;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_HWCAP2: usize = 26;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_PHDR: usize = 3;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_PHENT: usize = 4;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_PHNUM: usize = 5;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_EXECFN: usize = 31;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const EXECVE_AUXV_AT_SYSINFO_EHDR: usize = 33;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_AT_FDCWD: isize = -100;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_O_CREAT: usize = 0o100;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_O_EXCL: usize = 0o200;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_O_TRUNC: usize = 0o1000;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
fn is_stdio_fd(fd: usize) -> bool {
    matches!(fd, STDOUT_FD | STDERR_FD)
}

pub(super) fn sys_linux_shim(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    frame_ptr: *mut crate::kernel::syscalls::SyscallFrame,
) -> Option<usize> {
    dispatch::sys_linux_shim(syscall_id, arg1, arg2, arg3, arg4, arg5, arg6, frame_ptr)
}

#[cfg(not(feature = "linux_compat"))]
fn sys_linux_umask(new_mask: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::umask(new_mask as u32) as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = new_mask;
        0o022
    }
}

#[cfg(not(feature = "linux_compat"))]
fn sys_linux_arch_prctl(code: usize, addr: usize) -> usize {
    match code {
        linux::arch_prctl::ARCH_SET_FS => {
            let ret = sys_set_tls(addr);
            if ret == 0 {
                0
            } else {
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            }
        }
        linux::arch_prctl::ARCH_GET_FS => {
            with_user_write_words(addr, core::mem::size_of::<usize>(), 1, |out| {
                out[0] = sys_get_tls();
                0
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EACCES))
        }
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

#[cfg(not(feature = "linux_compat"))]
fn sys_linux_futex(uaddr: usize, op: usize, val: usize) -> usize {
    match op & LINUX_FUTEX_CMD_MASK {
        FUTEX_WAIT_OP => {
            let ret = sys_futex_wait(uaddr, val, 0);
            match ret {
                0 => 0,
                FUTEX_WAIT_VALUE_MISMATCH => {
                    linux_errno(crate::modules::posix_consts::errno::EAGAIN)
                }
                SYSCALL_ERR_USER_ACCESS_DENIED => {
                    linux_errno(crate::modules::posix_consts::errno::EFAULT)
                }
                SYSCALL_ERR_PERMISSION_DENIED => {
                    linux_errno(crate::modules::posix_consts::errno::EPERM)
                }
                SYSCALL_ERR_INVALID_ARG => linux_errno(crate::modules::posix_consts::errno::EINVAL),
                _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
            }
        }
        FUTEX_WAKE_OP => {
            let woke = sys_futex_wake(uaddr, val, 0);
            match woke {
                SYSCALL_ERR_USER_ACCESS_DENIED => {
                    linux_errno(crate::modules::posix_consts::errno::EFAULT)
                }
                SYSCALL_ERR_PERMISSION_DENIED => {
                    linux_errno(crate::modules::posix_consts::errno::EPERM)
                }
                SYSCALL_ERR_INVALID_ARG => linux_errno(crate::modules::posix_consts::errno::EINVAL),
                _ => woke,
            }
        }
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

#[cfg(test)]
mod tests;
