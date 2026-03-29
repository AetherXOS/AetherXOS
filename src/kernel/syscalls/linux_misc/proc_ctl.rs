#[cfg(not(feature = "linux_compat"))]
use super::{linux_errno, with_user_read_bytes, with_user_write_bytes};

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_prctl(
    option: usize,
    arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> usize {
    const PR_SET_NAME: usize = 15;
    const PR_GET_NAME: usize = 16;
    const PR_SET_PDEATHSIG: usize = 1;
    const PR_GET_PDEATHSIG: usize = 2;
    const PR_GET_DUMPABLE: usize = 3;
    const PR_SET_DUMPABLE: usize = 4;
    const PR_GET_KEEPCAPS: usize = 7;
    const PR_SET_KEEPCAPS: usize = 8;
    const PR_GET_SECCOMP: usize = 21;
    const PR_SET_SECCOMP: usize = 22;
    const PR_CAPBSET_READ: usize = 23;
    const PR_CAPBSET_DROP: usize = 24;
    const PR_SET_NO_NEW_PRIVS: usize = 38;
    const PR_GET_NO_NEW_PRIVS: usize = 39;

    match option {
        PR_SET_NAME => {
            if arg2 == 0 {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
            let name_bytes = match with_user_read_bytes(arg2, 16, |src| {
                let mut out = [0u8; 16];
                out.copy_from_slice(src);
                out
            }) {
                Ok(v) => v,
                Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
            };

            let nul = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_bytes.len());
            let new_name = alloc::string::String::from_utf8_lossy(&name_bytes[..nul]).into_owned();

            let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
            let tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
            if let Some(task) = crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid))
            {
                task.lock().name = new_name;
                0
            } else {
                linux_errno(crate::modules::posix_consts::errno::ESRCH)
            }
        }
        PR_GET_NAME => {
            if arg2 == 0 {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
            let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
            let tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
            let task_name = crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid))
                .map(|t| t.lock().name.clone())
                .unwrap_or_else(|| alloc::string::String::from("task"));

            with_user_write_bytes(arg2, 16, |dst| {
                dst.fill(0);
                let src = task_name.as_bytes();
                let len = core::cmp::min(src.len(), 15);
                if len > 0 {
                    dst[..len].copy_from_slice(&src[..len]);
                }
                0
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
        }
        PR_SET_PDEATHSIG => 0,
        PR_GET_PDEATHSIG => {
            if arg2 != 0 {
                let wrote = with_user_write_bytes(arg2, 4, |dst| {
                    dst.copy_from_slice(&0u32.to_ne_bytes());
                    0
                });
                if wrote.is_err() {
                    return linux_errno(crate::modules::posix_consts::errno::EFAULT);
                }
            }
            0
        }
        PR_GET_DUMPABLE => 1,
        PR_SET_DUMPABLE => 0,
        PR_GET_KEEPCAPS => 1,
        PR_SET_KEEPCAPS => 0,
        PR_GET_SECCOMP => 0,
        PR_SET_SECCOMP => match arg2 {
            0 | 1 => 0,
            2 => 0,
            _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        },
        PR_CAPBSET_READ => 0,
        PR_CAPBSET_DROP => 0,
        PR_SET_NO_NEW_PRIVS => 0,
        PR_GET_NO_NEW_PRIVS => 0,
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getparam(_pid: usize, param_ptr: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let priority = match crate::modules::posix::process::sched_getparam(_pid) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        return with_user_write_bytes(param_ptr, 4, |dst| {
            dst.copy_from_slice(&(priority as i32).to_ne_bytes());
            0
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    #[cfg(not(feature = "posix_process"))]
    {
        with_user_write_bytes(param_ptr, 4, |dst| {
            dst.copy_from_slice(&0u32.to_ne_bytes());
            0
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getscheduler(_pid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        return match crate::modules::posix::process::sched_getscheduler(_pid) {
            Ok(v) => v as usize,
            Err(err) => linux_errno(err.code()),
        };
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setparam(pid: usize, param_ptr: usize) -> usize {
    let priority = match with_user_read_bytes(param_ptr, 4, |src| {
        i32::from_ne_bytes([src[0], src[1], src[2], src[3]])
    }) {
        Ok(v) => v,
        Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
    };

    #[cfg(feature = "posix_process")]
    {
        return match crate::modules::posix::process::sched_setparam(pid, priority) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        };
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (pid, priority);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setscheduler(pid: usize, policy: usize, param_ptr: usize) -> usize {
    let priority = match with_user_read_bytes(param_ptr, 4, |src| {
        i32::from_ne_bytes([src[0], src[1], src[2], src[3]])
    }) {
        Ok(v) => v,
        Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
    };

    #[cfg(feature = "posix_process")]
    {
        return match crate::modules::posix::process::sched_setscheduler(
            pid,
            policy as i32,
            priority,
        ) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        };
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (pid, policy, priority);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getaffinity(
    _pid: usize,
    cpusetsize: usize,
    mask_ptr: usize,
) -> usize {
    let size = core::cmp::min(cpusetsize, 128);
    with_user_write_bytes(mask_ptr, size, |dst| {
        dst.fill(0);
        if !dst.is_empty() {
            dst[0] = 0xFF;
        }
        size
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setaffinity(
    _pid: usize,
    _cpusetsize: usize,
    _mask_ptr: usize,
) -> usize {
    0
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn prctl_get_pdeathsig_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_prctl(2, 0x1, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn prctl_set_name_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_prctl(15, 0x1, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn sched_getparam_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_sched_getparam(0, 0x1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn sched_setparam_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_sched_setparam(0, 0x1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
}
