use crate::kernel::syscalls::linux_errno;
use super::state::*;
use super::utils::*;

pub fn sys_linux_signalfd(fd: usize, mask_ptr: usize, sizemask: usize) -> usize {
    sys_linux_signalfd4(fd, mask_ptr, sizemask, 0)
}

pub fn sys_linux_signalfd4(
    fd: usize,
    mask_ptr: usize,
    sizemask: usize,
    flags: usize,
) -> usize {
    if sizemask != core::mem::size_of::<u64>() {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let mask = match read_u64_from_user(mask_ptr) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_signal")]
    {
        let raw_fd = fd as i32;
        let result = if raw_fd >= 0 {
            crate::modules::posix::signal::signalfd_reconfigure_errno(raw_fd as u32, mask, flags as i32)
        } else {
            crate::modules::posix::signal::signalfd_create_errno(mask, flags as i32)
        };
        match result {
            Ok(out_fd) => out_fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_signal"))]
    {
        let raw_fd = fd as i32;
        if raw_fd >= 0 {
            let mut table = SIGNALFD_MASK_BY_FD.lock();
            let Some(slot) = table.get_mut(&(raw_fd as u32)) else {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            };
            *slot = mask;
            return raw_fd as usize;
        }

        let id = NEXT_SIGNALFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let out_fd = (SIGNALFD_FD_BASE as u32).saturating_add(id);
        SIGNALFD_MASK_BY_FD.lock().insert(out_fd, mask);
        let _ = flags;
        out_fd as usize
    }
}
