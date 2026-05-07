#[cfg(not(feature = "linux_compat"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "linux_compat"))]
use spin::Mutex;

#[cfg(not(feature = "linux_compat"))]
pub(crate) static LINUX_FD_FLAGS: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());
#[cfg(not(feature = "linux_compat"))]
pub(crate) static LINUX_PIDFD_MAP: Mutex<BTreeMap<u32, LinuxPidFdEntry>> = Mutex::new(BTreeMap::new());
#[cfg(not(feature = "linux_compat"))]
pub(crate) static NEXT_SYNTH_PIDFD: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);

#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy)]
pub(crate) struct LinuxPidFdEntry {
    pub(crate) target_pid: usize,
    pub(crate) owner_tid: usize,
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) const LINUX_FD_CLOEXEC: usize = 0x1;
#[cfg(not(feature = "linux_compat"))]
pub(crate) const SYNTH_PIDFD_BASE: usize = 790_000;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_fd_get_descriptor_flags(fd: u32) -> usize {
    LINUX_FD_FLAGS.lock().get(&fd).copied().unwrap_or(0)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_fd_set_descriptor_flags(fd: u32, flags: usize) {
    let masked = flags & LINUX_FD_CLOEXEC;
    let mut table = LINUX_FD_FLAGS.lock();
    if masked == 0 {
        table.remove(&fd);
    } else {
        table.insert(fd, masked);
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_fd_clear_descriptor_flags(fd: u32) {
    LINUX_FD_FLAGS.lock().remove(&fd);
}

pub(crate) fn clear_linux_fd_flags(fd: u32) {
    linux_fd_clear_descriptor_flags(fd);
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_pidfd_clear(fd: u32) {
    LINUX_PIDFD_MAP.lock().remove(&fd);
}

pub(crate) fn clear_linux_pidfd_entry(fd: u32) {
    linux_pidfd_clear(fd);
}
#[cfg(not(feature = "linux_compat"))]
pub(crate) fn linux_fd_close_on_exec() {
    let mut to_close = alloc::vec::Vec::new();
    {
        let mut flags_table = LINUX_FD_FLAGS.lock();
        flags_table.retain(|&fd, &mut flags| {
            if (flags & LINUX_FD_CLOEXEC) != 0 {
                to_close.push(fd);
                false
            } else {
                true
            }
        });
    }

    for fd in to_close {
        // Note: We use the underlying posix close to avoid circular dependencies
        // and ensure the FD is truly released from the system.
        #[cfg(feature = "posix_fs")]
        {
            let _ = crate::modules::posix::fs::close(fd);
        }
        #[cfg(all(not(feature = "posix_fs"), feature = "posix_net"))]
        {
            // If it was a socket, we might need a different close, 
            // but usually close() works for both.
            let _ = crate::modules::posix::net::close(fd);
        }
    }
}
