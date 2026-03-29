use super::super::*;

const EFD_CLOEXEC: i32 = 0x80000;
const EFD_SEMAPHORE: i32 = 0x1;
const EFD_NONBLOCK: i32 = crate::modules::posix_consts::net::O_NONBLOCK as i32;
const EFD_ALLOWED_FLAGS: i32 = EFD_CLOEXEC | EFD_SEMAPHORE | EFD_NONBLOCK;

/// `eventfd(2)` — Create a file descriptor for event notification.
pub fn sys_linux_eventfd(initval: u32, flags: i32) -> usize {
    crate::require_posix_io!((initval, flags) => {
        if (flags & !EFD_ALLOWED_FLAGS) != 0 {
            return linux_inval();
        }

        match crate::modules::posix::io::eventfd_create_errno(initval, flags) {
            Ok(fd) => {
                if (flags & EFD_CLOEXEC) != 0 {
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
    })
}

pub fn sys_linux_eventfd2(initval: u32, flags: i32) -> usize {
    sys_linux_eventfd(initval, flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn eventfd_cloexec_sets_linux_descriptor_flag() {
        let fd = sys_linux_eventfd2(0, EFD_CLOEXEC) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
    }

    #[test_case]
    fn eventfd_rejects_unknown_flags() {
        assert_eq!(sys_linux_eventfd2(0, 0x40), linux_inval());
    }
}
