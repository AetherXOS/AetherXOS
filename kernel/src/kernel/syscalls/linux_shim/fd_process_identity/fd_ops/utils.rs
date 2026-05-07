#[cfg(not(feature = "linux_compat"))]
use super::storage::{LinuxPidFdEntry, LINUX_PIDFD_MAP};
#[cfg(not(feature = "linux_compat"))]
use super::super::super::super::linux_errno;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_current_tid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_task_exists(pid: usize) -> bool {
    crate::kernel::task::get_task(crate::interfaces::task::TaskId(pid)).is_some()
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_pidfd_entry_for_caller(pidfd: usize) -> Result<LinuxPidFdEntry, usize> {
    let entry = LINUX_PIDFD_MAP
        .lock()
        .get(&(pidfd as u32))
        .copied()
        .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EBADF))?;

    if linux_current_tid() != entry.owner_tid {
        return Err(linux_errno(crate::modules::posix_consts::errno::EPERM));
    }
    if !linux_task_exists(entry.target_pid) {
        return Err(linux_errno(crate::modules::posix_consts::errno::ESRCH));
    }

    Ok(entry)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_pidfd_getfd_access_allowed(caller_tid: usize, target_pid: usize) -> bool {
    caller_tid == target_pid || caller_tid == 1
}
