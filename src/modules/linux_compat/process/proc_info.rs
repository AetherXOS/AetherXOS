use super::super::*;
use crate::kernel::syscalls::with_user_write_bytes;

const LINUX_SIGNAL_MAX: i32 = 64;
const PRLIMIT_FALLBACK_SOFT: u64 = 1024;
const PRLIMIT_FALLBACK_HARD: u64 = 4096;
const DEFAULT_GROUP_COUNT: usize = 1;

/// `getpid(2)` — Process ID (TGID in thread context).
pub fn sys_linux_getpid() -> usize {
    crate::modules::posix::process::getpid() as usize
}

/// `getppid(2)` — Parent Process ID.
pub fn sys_linux_getppid() -> usize {
    crate::modules::posix::process::getppid() as usize
}

/// `gettid(2)` — Thread ID.
pub fn sys_linux_gettid() -> usize {
    if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
        return cpu.current_task.load(core::sync::atomic::Ordering::Relaxed) as usize;
    }
    0
}

/// `tgkill(2)` — Deliver signal to specific thread in specific group.
pub fn sys_linux_tgkill(tgid: i32, tid: i32, sig: i32) -> usize {
    if tgid <= 0 || tid <= 0 {
        return linux_inval();
    }
    if sig < 0 || sig > LINUX_SIGNAL_MAX {
        return linux_inval();
    }

    let task_id = crate::interfaces::task::TaskId(tid as usize);
    if let Some(task) = crate::kernel::task::get_task(task_id) {
        let mut locked = task.lock();
        if let Some(tg) = locked.process_id {
            if tg.0 as i32 != tgid {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
        }
        if sig == 0 {
            return 0;
        }
        locked.pending_signals |= 1u64 << (sig - 1);
        0
    } else {
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

pub fn sys_linux_tkill(tid: i32, sig: i32) -> usize {
    if tid <= 0 {
        return linux_inval();
    }
    let task_id = crate::interfaces::task::TaskId(tid as usize);
    if let Some(task) = crate::kernel::task::get_task(task_id) {
        if sig == 0 {
            return 0;
        }
        task.lock().pending_signals |= 1u64 << (sig - 1);
        0
    } else {
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

/// `prlimit64(2)` — Get/Set resource limits.
pub fn sys_linux_prlimit64(
    pid: i32,
    resource: usize,
    new_limit: UserPtr<u64>,
    old_limit: UserPtr<u64>,
) -> usize {
    crate::require_posix_process!((pid, resource, new_limit, old_limit) => {
        let target_pid = if pid == 0 { sys_linux_getpid() } else { pid as usize };

        // 1. Get old limits
        if !old_limit.is_null() {
            let res = crate::modules::posix::process::prlimit(target_pid, resource as i32, None);
            if let Ok((soft, hard)) = res {
                let _ = old_limit.write(&soft);
                let _ = old_limit.offset(1).write(&hard);
            } else {
                // Fallback dummy
                let _ = old_limit.write(&PRLIMIT_FALLBACK_SOFT);
                let _ = old_limit.offset(1).write(&PRLIMIT_FALLBACK_HARD);
            }
        }

        // 2. Set new limits
        if !new_limit.is_null() {
            let soft = match new_limit.read() { Ok(v) => v, Err(e) => return e };
            let hard = match new_limit.offset(1).read() { Ok(v) => v, Err(e) => return e };
            let _ = crate::modules::posix::process::prlimit(target_pid, resource as i32, Some((soft, hard)));
        }
        0
    })
}

pub fn sys_linux_getrlimit(resource: usize, rlim: UserPtr<u64>) -> usize {
    sys_linux_prlimit64(0, resource, UserPtr::new(0), rlim)
}

pub fn sys_linux_getuid() -> usize {
    crate::modules::posix::process::getuid() as usize
}
pub fn sys_linux_geteuid() -> usize {
    crate::modules::posix::process::geteuid() as usize
}
pub fn sys_linux_getgid() -> usize {
    crate::modules::posix::process::getgid() as usize
}
pub fn sys_linux_getegid() -> usize {
    crate::modules::posix::process::getegid() as usize
}

pub fn sys_linux_getgroups(size: usize, list: UserPtr<u32>) -> usize {
    if size == 0 {
        return DEFAULT_GROUP_COUNT;
    }
    if list.is_null() {
        return linux_fault();
    }
    let _ = list.write(&0);
    DEFAULT_GROUP_COUNT
}

pub fn sys_linux_getpgid(pid: usize) -> usize {
    if pid == 0 {
        return sys_linux_getpid();
    }
    pid
}
pub fn sys_linux_getsid(pid: usize) -> usize {
    if pid == 0 {
        return sys_linux_getpid();
    }
    pid
}

pub fn sys_linux_getcpu(
    cpu_ptr: UserPtr<u32>,
    node_ptr: UserPtr<u32>,
    _tcache: UserPtr<u8>,
) -> usize {
    crate::require_posix_process!((cpu_ptr, node_ptr, _tcache) => {
        let (cpu, node) = match crate::modules::posix::process::getcpu() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        if !cpu_ptr.is_null() {
            if let Err(e) = cpu_ptr.write(&cpu) {
                return e;
            }
        }
        if !node_ptr.is_null() {
            if let Err(e) = node_ptr.write(&node) {
                return e;
            }
        }
        0
    })
}

pub fn sys_linux_getitimer(_which: usize, value: UserPtr<u8>) -> usize {
    if value.is_null() {
        return linux_fault();
    }
    let _ = with_user_write_bytes(value.addr, 16, |dst| {
        dst.fill(0);
        0
    });
    0
}
pub fn sys_linux_setitimer(which: usize, value: UserPtr<u8>, old: UserPtr<u8>) -> usize {
    if value.is_null() {
        return linux_fault();
    }
    if !old.is_null() {
        sys_linux_getitimer(which, old);
    }
    0
}

pub fn sys_linux_getrusage(who: i32, ru: UserPtr<LinuxRusage>) -> usize {
    if ru.is_null() {
        return linux_fault();
    }

    crate::require_posix_process!((who, ru) => {
        let usage = match crate::modules::posix::process::getrusage(who) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match ru.write(&fill_linux_rusage(usage)) {
            Ok(()) => 0,
            Err(e) => e,
        }
    })
}
