use super::*;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_set_robust_list(head: usize, len: usize) -> usize {
    let expected = crate::generated_consts::LINUX_ROBUST_LIST_HEAD_SIZE;
    if len != expected {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    crate::kernel::syscalls::set_robust_list_for_tid(current_tid, head, len);
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_get_robust_list(pid: usize, head_ptr: usize, len_ptr: usize) -> usize {
    if head_ptr == 0 || len_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    let target_tid = if pid == 0 { current_tid } else { pid };

    if crate::kernel::task::get_task(crate::interfaces::task::TaskId(target_tid)).is_none() {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    }

    let default_len = crate::generated_consts::LINUX_ROBUST_LIST_HEAD_SIZE;
    let (head, len) =
        crate::kernel::syscalls::robust_list_for_tid(target_tid).unwrap_or((0, default_len));

    let wr_head = with_user_write_words(head_ptr, core::mem::size_of::<usize>(), 1, |out| {
        out[0] = head;
        0usize
    });
    if wr_head.is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    let wr_len = with_user_write_words(len_ptr, core::mem::size_of::<usize>(), 1, |out| {
        out[0] = len;
        0usize
    });
    if wr_len.is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    0
}
