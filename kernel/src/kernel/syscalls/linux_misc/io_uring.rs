use crate::kernel::syscalls::linux_errno;
use super::state::*;

pub fn sys_linux_io_uring_setup(entries: usize, params_ptr: usize) -> usize {
    if entries == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if params_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let id = NEXT_IO_URING_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    IO_URING_IDS.lock().insert(id);
    IO_URING_FD_BASE.saturating_add(id as usize)
}

pub fn sys_linux_io_uring_enter(
    fd: usize,
    to_submit: usize,
    min_complete: usize,
    flags: usize,
    sig_ptr: usize,
    sigsz: usize,
) -> usize {
    let _ = (to_submit, min_complete, flags, sig_ptr, sigsz);
    if fd < IO_URING_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (fd - IO_URING_FD_BASE) as u32;
    if !IO_URING_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_io_uring_register(
    fd: usize,
    opcode: usize,
    arg_ptr: usize,
    nr_args: usize,
) -> usize {
    let _ = (opcode, arg_ptr, nr_args);
    if fd < IO_URING_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (fd - IO_URING_FD_BASE) as u32;
    if !IO_URING_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}
