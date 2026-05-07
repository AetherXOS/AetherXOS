pub mod duplication;
pub mod fcntl;
pub mod pidfd;
pub mod storage;
pub mod utils;

#[allow(unused_imports)]
pub use duplication::*;
#[allow(unused_imports)]
pub use fcntl::*;
#[allow(unused_imports)]
pub use pidfd::*;
#[allow(unused_imports)]
pub use storage::*;


#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;
    use crate::kernel::syscalls::linux_errno;
    use crate::kernel::syscalls::linux_shim::fd_process_identity::fd_ops::utils::linux_pidfd_getfd_access_allowed;

    #[test_case]
    fn socket_fd_close_falls_back_to_network_layer() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        assert_eq!(
            crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize),
            0
        );
        assert_eq!(
            crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize),
            0
        );
    }

    #[test_case]
    fn socket_fd_dup_and_dup3_preserve_linux_descriptor_flags() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        let duped = sys_linux_dup(fd_a as usize);
        assert!(duped > 2);

        let target_fd = duped + 17;
        assert_eq!(sys_linux_dup3(fd_a as usize, target_fd, 0x80000), target_fd);
        assert_eq!(
            linux_fd_get_descriptor_flags(target_fd as u32) & LINUX_FD_CLOEXEC,
            LINUX_FD_CLOEXEC
        );

        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(duped);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(target_fd);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize);
    }

    #[test_case]
    fn socket_fd_fcntl_status_flags_use_network_backend() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        let getfl = sys_linux_fcntl(fd_a as usize, 3, 0);
        assert_ne!(
            getfl,
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );

        assert_eq!(sys_linux_fcntl(fd_a as usize, 4, 0), 0);

        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize);
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_fs"))]
mod pidfd_tests {
    use super::*;
    use crate::kernel::syscalls::linux_errno;
    use crate::kernel::syscalls::linux_shim::fd_process_identity::fd_ops::utils::linux_pidfd_getfd_access_allowed;

    #[test_case]
    fn pidfd_open_rejects_zero_pid() {
        assert_eq!(
            sys_linux_pidfd_open(0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn pidfd_send_signal_rejects_unknown_pidfd() {
        assert_eq!(
            sys_linux_pidfd_send_signal(424242, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }

    #[test_case]
    fn pidfd_open_rejects_missing_task() {
        assert_eq!(
            sys_linux_pidfd_open(9_999_999, 0),
            linux_errno(crate::modules::posix_consts::errno::ESRCH)
        );
    }

    #[test_case]
    fn pidfd_getfd_rejects_nonzero_flags() {
        let pidfd = sys_linux_pidfd_open(1, 0);
        if pidfd >= linux_errno(crate::modules::posix_consts::errno::MAX_ERRNO) {
            return;
        }
        let rc = sys_linux_pidfd_getfd(pidfd, 0, 1);
        assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::EINVAL));
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(pidfd);
    }

    #[test_case]
    fn pidfd_getfd_access_matrix_allows_self_and_supervisor() {
        assert!(linux_pidfd_getfd_access_allowed(77, 77));
        assert!(linux_pidfd_getfd_access_allowed(1, 77));
        assert!(!linux_pidfd_getfd_access_allowed(33, 77));
    }
}
