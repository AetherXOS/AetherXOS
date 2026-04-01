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
