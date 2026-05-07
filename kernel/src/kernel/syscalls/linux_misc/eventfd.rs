use crate::kernel::syscalls::linux_errno;

pub fn sys_linux_eventfd(initval: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_io")]
    {
        match crate::modules::posix::io::eventfd_create_errno(initval as u32, flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_io"))]
    {
        let id = NEXT_EVENTFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (EVENTFD_FD_BASE as u32).saturating_add(id);
        EVENTFD_STATE_BY_FD.lock().insert(fd, initval as u64);
        let _ = flags;
        fd as usize
    }
}

pub fn sys_linux_eventfd2(initval: usize, flags: usize) -> usize {
    sys_linux_eventfd(initval, flags)
}
