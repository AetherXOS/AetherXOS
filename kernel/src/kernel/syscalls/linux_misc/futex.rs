use crate::kernel::syscalls::linux_errno;
use super::types::*;
use super::utils::*;

pub fn sys_linux_futex_waitv(
    waiters_ptr: usize,
    nr_futexes: usize,
    flags: usize,
    timeout_ptr: usize,
) -> usize {
    if flags != 0 || waiters_ptr == 0 || nr_futexes == 0 || nr_futexes > crate::generated_consts::LINUX_FUTEX_WAITV_MAX {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let item_sz = core::mem::size_of::<LinuxFutexWaitVCompat>();
    for i in 0..nr_futexes {
        let ptr = match waiters_ptr.checked_add(i.saturating_mul(item_sz)) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
        };
        let waiter = match read_user_struct::<LinuxFutexWaitVCompat>(ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if waiter.__reserved != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
    }

    if timeout_ptr != 0 {
        let ts = match read_user_struct::<LinuxTimespecCompat>(timeout_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if !validate_timespec_compat(ts) {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        return linux_errno(crate::modules::posix_consts::errno::ETIMEDOUT);
    }

    linux_errno(crate::modules::posix_consts::errno::EAGAIN)
}
