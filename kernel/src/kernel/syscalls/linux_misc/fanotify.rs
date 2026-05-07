use crate::kernel::syscalls::linux_errno;
use super::state::*;
use super::types::*;
use super::utils::*;

pub fn sys_linux_fanotify_init(flags: usize, event_f_flags: usize) -> usize {
    const FAN_CLASS_NOTIF: usize = 0x0000;
    const FAN_CLASS_CONTENT: usize = 0x0004;
    const FAN_CLASS_PRE_CONTENT: usize = 0x0008;
    const FAN_CLOEXEC: usize = 0x0000_0001;
    const FAN_NONBLOCK: usize = 0x0000_0002;
    const FAN_UNLIMITED_QUEUE: usize = 0x0000_0010;
    const FAN_UNLIMITED_MARKS: usize = 0x0000_0020;
    const FAN_REPORT_FID: usize = 0x0000_0200;

    let class_bits = flags & (FAN_CLASS_CONTENT | FAN_CLASS_PRE_CONTENT);
    if class_bits == (FAN_CLASS_CONTENT | FAN_CLASS_PRE_CONTENT) {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let allowed_init_flags = FAN_CLASS_NOTIF
        | FAN_CLASS_CONTENT
        | FAN_CLASS_PRE_CONTENT
        | FAN_CLOEXEC
        | FAN_NONBLOCK
        | FAN_UNLIMITED_QUEUE
        | FAN_UNLIMITED_MARKS
        | FAN_REPORT_FID;
    if (flags & !allowed_init_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let allowed_event_f_flags =
        crate::modules::posix_consts::fs::O_RDONLY as usize
            | crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CLOEXEC
            | crate::modules::posix_consts::net::O_NONBLOCK as usize;
    if (event_f_flags & !allowed_event_f_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let id = NEXT_FANOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let path = alloc::format!("/.fanotify-{}", id);
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => {
                if (flags & FAN_CLOEXEC) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_descriptor_flags(
                        fd,
                        crate::modules::posix_consts::net::FD_CLOEXEC,
                    );
                }
                if (flags & FAN_NONBLOCK) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    );
                }
                FANOTIFY_MARKS_BY_FD.lock().insert(fd, alloc::vec::Vec::new());
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_FANOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let _ = (flags, event_f_flags);
        FANOTIFY_MARKS_BY_FD
            .lock()
            .entry((FANOTIFY_FD_BASE as u32).saturating_add(id))
            .or_default();
        FANOTIFY_FD_BASE.saturating_add(id as usize)
    }
}

pub fn sys_linux_fanotify_mark(
    fanotify_fd: usize,
    flags: usize,
    mask: usize,
    dirfd: isize,
    path_ptr: usize,
) -> usize {
    const FAN_MARK_ADD: usize = 0x0000_0001;
    const FAN_MARK_REMOVE: usize = 0x0000_0002;
    const FAN_MARK_FLUSH: usize = 0x0000_0080;

    let op_count = usize::from((flags & FAN_MARK_ADD) != 0)
        + usize::from((flags & FAN_MARK_REMOVE) != 0)
        + usize::from((flags & FAN_MARK_FLUSH) != 0);
    if op_count != 1 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let fd = fanotify_fd as u32;
    let mut marks = FANOTIFY_MARKS_BY_FD.lock();
    let Some(fd_marks) = marks.get_mut(&fd) else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };

    if (flags & FAN_MARK_FLUSH) != 0 {
        fd_marks.clear();
        return 0;
    }

    if path_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if mask == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let path = match read_user_c_string_compat(path_ptr, crate::config::KernelConfig::syscall_max_path_len()) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if (flags & FAN_MARK_ADD) != 0 {
        fd_marks.push(FanotifyMarkState {
            mask,
            dirfd,
            path,
        });
        return 0;
    }

    if let Some(idx) = fd_marks
        .iter()
        .position(|entry| entry.path == path && entry.dirfd == dirfd && entry.mask == mask)
    {
        fd_marks.swap_remove(idx);
        return 0;
    }

    linux_errno(crate::modules::posix_consts::errno::ENOENT)
}
