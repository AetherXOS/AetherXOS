// Linux syscall compatibility shim (non-linux_compat feature path).
// All `#[cfg(not(feature = "linux_compat"))]` Linux syscall implementations live here.
// In production builds (with `linux_compat` feature), this module is not compiled;
// the full linux_compat layer in `src/modules/linux_compat/` handles dispatch instead.

use super::*;
#[cfg(not(feature = "linux_compat"))]
mod dispatch;
#[cfg(not(feature = "linux_compat"))]
mod fd_process_identity;
#[cfg(not(feature = "linux_compat"))]
mod fs;
#[cfg(not(feature = "linux_compat"))]
mod ipc;
#[cfg(not(feature = "linux_compat"))]
mod memory;

#[cfg(not(feature = "linux_compat"))]
pub(crate) mod net;
pub mod process;
#[cfg(not(feature = "linux_compat"))]
mod signal;
#[cfg(not(feature = "linux_compat"))]
mod task_time;
pub(crate) mod util;
#[cfg(not(feature = "linux_compat"))]
pub use util::{read_user_pod, write_user_pod, LinuxRUsage};
#[cfg(not(feature = "linux_compat"))]

const LINUX_FUTEX_CMD_MASK: usize = 0x7f;
#[cfg(not(feature = "linux_compat"))]

const EXECVE_MAX_VECTOR_ITEMS: usize = 256;
#[cfg(not(feature = "linux_compat"))]

const EXECVE_STACK_BYTES: u64 = 2 * crate::interfaces::memory::PAGE_SIZE_4K as u64;

const EXECVE_AUXV_AT_NULL: usize = 0;

const EXECVE_AUXV_AT_ENTRY: usize = 9;

const EXECVE_AUXV_AT_PAGESZ: usize = 6;

const EXECVE_AUXV_AT_BASE: usize = 7;

const EXECVE_AUXV_AT_FLAGS: usize = 8;

const EXECVE_AUXV_AT_UID: usize = 11;

const EXECVE_AUXV_AT_EUID: usize = 12;

const EXECVE_AUXV_AT_GID: usize = 13;

const EXECVE_AUXV_AT_EGID: usize = 14;

const EXECVE_AUXV_AT_SECURE: usize = 23;

const EXECVE_AUXV_AT_RANDOM: usize = 25;

const EXECVE_AUXV_AT_HWCAP: usize = 16;

const EXECVE_AUXV_AT_CLKTCK: usize = 17;

const EXECVE_AUXV_AT_PLATFORM: usize = 15;

const EXECVE_AUXV_AT_HWCAP2: usize = 26;

const EXECVE_AUXV_AT_PHDR: usize = 3;

const EXECVE_AUXV_AT_PHENT: usize = 4;

const EXECVE_AUXV_AT_PHNUM: usize = 5;

const EXECVE_AUXV_AT_EXECFN: usize = 31;

const EXECVE_AUXV_AT_SYSINFO_EHDR: usize = 33;
#[cfg(not(feature = "linux_compat"))]

pub(crate) const LINUX_AT_FDCWD: isize = -100;
#[cfg(not(feature = "linux_compat"))]

pub(crate) const LINUX_O_CREAT: usize = 0o100;
#[cfg(not(feature = "linux_compat"))]

pub(crate) const LINUX_O_EXCL: usize = 0o200;
#[cfg(not(feature = "linux_compat"))]

pub(crate) const LINUX_O_TRUNC: usize = 0o1000;
#[cfg(not(feature = "linux_compat"))]

pub(crate) const LINUX_O_APPEND: usize = 0o2000;
#[cfg(not(feature = "linux_compat"))]

fn is_stdio_fd(fd: usize) -> bool {
    matches!(fd, STDOUT_FD | STDERR_FD)
}

#[cfg(not(feature = "linux_compat"))]
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
fn sys_linux_futex(uaddr: usize, op: usize, _val: usize, _arg4: usize, uaddr2: usize, arg6: usize) -> usize {
    const FUTEX_WAIT: usize = 0;
    const FUTEX_WAKE: usize = 1;
    const FUTEX_REQUEUE: usize = 3;
    const FUTEX_CMP_REQUEUE: usize = 4;
    const FUTEX_WAIT_BITSET: usize = 9;
    const FUTEX_WAKE_BITSET: usize = 10;
    const FUTEX_PRIVATE_FLAG: usize = 128;

    let cmd = op & !FUTEX_PRIVATE_FLAG;
    let _key = crate::kernel::syscalls::ipc_control::futex_key_from_ptr_or_hint(uaddr, 0);

    match cmd {
        FUTEX_WAIT | FUTEX_WAIT_BITSET => {
            let _mask = if cmd == FUTEX_WAIT_BITSET { arg6 as u32 } else { 0xFFFF_FFFF };
            let _observed = match read_user_pod::<u32>(uaddr) {
                Ok(v) => v,
                Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
            };

            #[cfg(feature = "ipc_futex")]
            {
                let ret = crate::modules::ipc::futex::FUTEX_MANAGER.wait_bitset(key, observed, val as u32, mask);
                match ret {
                    crate::modules::ipc::futex::FutexWaitResult::Enqueued => 0,
                    crate::modules::ipc::futex::FutexWaitResult::ValueMismatch => {
                        linux_errno(crate::modules::posix_consts::errno::EAGAIN)
                    }
                }
            }
            #[cfg(not(feature = "ipc_futex"))]
            {
                0
            }
        }
        FUTEX_WAKE | FUTEX_WAKE_BITSET => {
            let _mask = if cmd == FUTEX_WAKE_BITSET { arg6 as u32 } else { 0xFFFF_FFFF };
            #[cfg(feature = "ipc_futex")]
            {
                crate::modules::ipc::futex::FUTEX_MANAGER.wake_bitset(key, val, mask)
            }
            #[cfg(not(feature = "ipc_futex"))]
            {
                0
            }
        }
        FUTEX_REQUEUE | FUTEX_CMP_REQUEUE => {
            let _key2 = crate::kernel::syscalls::ipc_control::futex_key_from_ptr_or_hint(uaddr2, 0);

            if cmd == FUTEX_CMP_REQUEUE {
                let observed = match read_user_pod::<u32>(uaddr) {
                    Ok(v) => v,
                    Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
                };
                if observed != (arg6 as u32) {
                    return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
                }
            }

            #[cfg(feature = "ipc_futex")]
            {
                crate::modules::ipc::futex::FUTEX_MANAGER.requeue(key, val, key2, val2)
            }
            #[cfg(not(feature = "ipc_futex"))]
            {
                0
            }
        }
        _ => linux_errno(crate::modules::posix_consts::errno::ENOSYS),
    }
}


#[cfg(test)]
mod tests;
