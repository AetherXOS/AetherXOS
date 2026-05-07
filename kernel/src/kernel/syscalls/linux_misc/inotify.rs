use crate::kernel::syscalls::linux_errno;
use super::utils::*;

pub fn sys_linux_inotify_init() -> usize {
    sys_linux_inotify_init1(0)
}

pub fn sys_linux_inotify_init1(flags: usize) -> usize {
    let allowed_flags = 0x0000_0800usize | 0x0008_0000usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_init(flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_INOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (INOTIFY_FD_BASE as u32).saturating_add(id);
        INOTIFY_WATCHES_BY_FD.lock().insert(fd, alloc::vec::Vec::new());
        let _ = flags;
        fd as usize
    }
}

pub fn sys_linux_inotify_add_watch(fd: usize, path_ptr: usize, mask: usize) -> usize {
    let path = match read_user_c_string_compat(path_ptr, crate::config::KernelConfig::syscall_max_path_len()) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_add_watch(fd as u32, &path, mask as u32) {
            Ok(wd) => wd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let fd = fd as u32;
        let mut watches = INOTIFY_WATCHES_BY_FD.lock();
        let Some(list) = watches.get_mut(&fd) else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        let wd = NEXT_INOTIFY_WD.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        list.push(InotifyWatchState {
            wd,
            path,
            mask: mask as u32,
        });
        wd as usize
    }
}

pub fn sys_linux_inotify_rm_watch(fd: usize, wd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_rm_watch(fd as u32, wd as i32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let fd = fd as u32;
        let mut watches = INOTIFY_WATCHES_BY_FD.lock();
        let Some(list) = watches.get_mut(&fd) else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        let target = wd as i32;
        let Some(index) = list.iter().position(|entry| entry.wd == target) else {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        };
        let removed = list.swap_remove(index);
        let _ = (removed.path, removed.mask);
        0
    }
}
