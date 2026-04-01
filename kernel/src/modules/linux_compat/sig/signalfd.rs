use super::super::*;
use crate::modules::posix::signal::SigSet;

const SFD_CLOEXEC: i32 = 0x80000;
const SFD_NONBLOCK: i32 = crate::modules::posix_consts::net::O_NONBLOCK as i32;
const SFD_ALLOWED_FLAGS: i32 = SFD_CLOEXEC | SFD_NONBLOCK;

pub fn sys_linux_signalfd(fd: Fd, mask: UserPtr<SigSet>, sizemask: usize) -> usize {
    sys_linux_signalfd_impl(fd, mask, sizemask, 0)
}

pub fn sys_linux_signalfd4(fd: Fd, mask: UserPtr<SigSet>, sizemask: usize, flags: i32) -> usize {
    sys_linux_signalfd_impl(fd, mask, sizemask, flags)
}

fn sys_linux_signalfd_impl(fd: Fd, mask: UserPtr<SigSet>, sizemask: usize, flags: i32) -> usize {
    if sizemask != 8 {
        return linux_inval();
    }
    if (flags & !SFD_ALLOWED_FLAGS) != 0 {
        return linux_inval();
    }
    let sigset = match mask.read() {
        Ok(s) => s,
        Err(_) => return linux_inval(),
    };

    let result = if fd.as_i32() >= 0 {
        crate::modules::posix::signal::signalfd_reconfigure_errno(fd.as_u32(), sigset, flags)
    } else {
        crate::modules::posix::signal::signalfd_create_errno(sigset, flags)
    };

    match result {
        Ok(fd) => {
            if (flags & SFD_CLOEXEC) != 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn signalfd4_cloexec_sets_linux_descriptor_flag() {
        let mask = 0u64;
        let fd = sys_linux_signalfd4(
            Fd(-1),
            UserPtr::new((&mask as *const u64) as usize),
            8,
            SFD_CLOEXEC,
        ) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
    }

    #[test_case]
    fn signalfd4_reuses_existing_fd_when_provided() {
        let mask_a = 0u64;
        let fd = sys_linux_signalfd4(Fd(-1), UserPtr::new((&mask_a as *const u64) as usize), 8, 0)
            as u32;
        let mask_b = 0x4u64;
        let reused = sys_linux_signalfd4(
            Fd(fd as i32),
            UserPtr::new((&mask_b as *const u64) as usize),
            8,
            SFD_CLOEXEC,
        ) as u32;
        assert_eq!(reused, fd);
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
    }

    #[test_case]
    fn signalfd_reuses_existing_fd_when_provided() {
        let mask_a = 0u64;
        let fd =
            sys_linux_signalfd(Fd(-1), UserPtr::new((&mask_a as *const u64) as usize), 8) as u32;
        let mask_b = 0x8u64;
        let reused = sys_linux_signalfd(
            Fd(fd as i32),
            UserPtr::new((&mask_b as *const u64) as usize),
            8,
        ) as u32;
        assert_eq!(reused, fd);
    }

    #[test_case]
    fn signalfd4_rejects_unknown_flags() {
        let mask = 0u64;
        assert_eq!(
            sys_linux_signalfd4(
                Fd(-1),
                UserPtr::new((&mask as *const u64) as usize),
                8,
                0x40
            ),
            linux_inval()
        );
    }
}
