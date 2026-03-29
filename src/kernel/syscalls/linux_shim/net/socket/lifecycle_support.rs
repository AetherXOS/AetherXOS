#[cfg(feature = "posix_net")]
use crate::kernel::syscalls::{linux_errno, with_user_write_words};

#[cfg(feature = "posix_net")]
pub(super) fn accept_socket_with_flags(fd: usize, flags_raw: i32) -> Result<u32, usize> {
    let accepted = if flags_raw == 0 {
        crate::modules::libnet::posix_accept_errno(fd as u32)
    } else {
        crate::modules::posix::net::accept4_raw_errno(fd as u32, flags_raw)
            .map_err(|err| crate::modules::libnet::PosixErrno::from_code(err.code()))
    };

    accepted.map_err(|err| linux_errno(err.code()))
}

#[cfg(feature = "posix_net")]
pub(super) fn write_socketpair_fds(sv_ptr: usize, fd0: u32, fd1: u32) -> Result<(), usize> {
    with_user_write_words(sv_ptr, core::mem::size_of::<usize>() * 2, 2, |out| {
        out[0] = fd0 as usize;
        out[1] = fd1 as usize;
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn accept_helper_rejects_invalid_flags_before_backend_lookup() {
        assert_eq!(
            accept_socket_with_flags(0, crate::modules::posix_consts::net::SOCK_NONBLOCK | 0x4000),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn socketpair_writer_reports_efault_for_invalid_output_pointer() {
        assert_eq!(
            write_socketpair_fds(0, 1, 2),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }
}
