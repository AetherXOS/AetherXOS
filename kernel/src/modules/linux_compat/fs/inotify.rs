use super::super::*;

const INOTIFY_ALLOWED_FLAGS: i32 =
    crate::modules::posix_consts::net::O_NONBLOCK as i32 | linux::open_flags::O_CLOEXEC as i32;

pub fn sys_linux_inotify_init() -> usize {
    match crate::modules::posix::fs::inotify_init(0) {
        Ok(fd) => fd as usize,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_inotify_init1(flags: i32) -> usize {
    if (flags & !INOTIFY_ALLOWED_FLAGS) != 0 {
        return linux_inval();
    }
    match crate::modules::posix::fs::inotify_init(flags) {
        Ok(fd) => {
            if (flags & linux::open_flags::O_CLOEXEC as i32) != 0 {
                crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                    fd,
                    crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                );
            } else {
                crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(fd);
            }
            fd as usize
        }
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_inotify_add_watch(fd: Fd, path: UserPtr<u8>, mask: u32) -> usize {
    let path_str = match path.as_str() {
        Ok(s) => s,
        Err(_) => return linux_inval(),
    };
    match crate::modules::posix::fs::inotify_add_watch(fd.as_u32(), path_str, mask) {
        Ok(wd) => wd as usize,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_inotify_rm_watch(fd: Fd, wd: i32) -> usize {
    match crate::modules::posix::fs::inotify_rm_watch(fd.as_u32(), wd) {
        Ok(()) => 0,
        Err(e) => linux_errno(e.code()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn inotify_init1_sets_cloexec_and_nonblock() {
        let fd = sys_linux_inotify_init1(
            crate::modules::posix_consts::net::O_NONBLOCK as i32
                | linux::open_flags::O_CLOEXEC as i32,
        ) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
        assert_eq!(
            crate::modules::posix::fs::fcntl_get_status_flags(fd).expect("status flags")
                & crate::modules::posix_consts::net::O_NONBLOCK,
            crate::modules::posix_consts::net::O_NONBLOCK
        );
    }

    #[test_case]
    fn inotify_init1_rejects_unknown_flags() {
        assert_eq!(sys_linux_inotify_init1(0x40), linux_inval());
    }
}
