#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getpid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getpid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        1
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getppid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getppid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_gettid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::gettid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_set_tid_address(tidptr: usize) -> usize {
    let tid = sys_linux_gettid();
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        task_arc.lock().clear_child_tid = tidptr;
    }
    tid
}
#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getuid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        task_arc.lock().security_ctx.ruid as usize
    } else {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getgid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        task_arc.lock().security_ctx.rgid as usize
    } else {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_geteuid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        task_arc.lock().security_ctx.euid as usize
    } else {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getegid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        task_arc.lock().security_ctx.egid as usize
    } else {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_setuid(uid: u32) -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        let mut task = task_arc.lock();
        let is_root = task.security_ctx.is_root();

        if is_root {
            task.security_ctx.ruid = uid;
            task.security_ctx.euid = uid;
            task.security_ctx.suid = uid;
            0
        } else if uid == task.security_ctx.ruid || uid == task.security_ctx.suid {
            task.security_ctx.euid = uid;
            0
        } else {
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EPERM)
        }
    } else {
        crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_setgid(gid: u32) -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    if let Some(task_arc) =
        crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
    {
        let mut task = task_arc.lock();
        let is_root = task.security_ctx.is_root();

        if is_root {
            task.security_ctx.rgid = gid;
            task.security_ctx.egid = gid;
            task.security_ctx.sgid = gid;
            0
        } else if gid == task.security_ctx.rgid || gid == task.security_ctx.sgid {
            task.security_ctx.egid = gid;
            0
        } else {
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EPERM)
        }
    } else {
        crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getrusage(who: i32, usage_ptr: usize) -> usize {
    use crate::kernel::syscalls::linux_shim::util::{write_user_pod, LinuxRUsage};
    
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::getrusage(who) {
            Ok(ru) => {
                let usage = LinuxRUsage::from(ru);
                match write_user_pod(usage_ptr, &usage) {
                    Ok(_) => 0,
                    Err(_) => crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EFAULT),
                }
            }
            Err(e) => crate::kernel::syscalls::linux_errno(e as i32),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (who, usage_ptr);
        crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}
