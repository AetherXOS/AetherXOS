use super::*;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;
#[cfg(feature = "posix_fs")]
use crate::kernel::syscalls::linux_shim::fs::support::LINUX_AT_EMPTY_PATH;

#[test_case]
fn faccessat_invalid_path_pointer_returns_efault() {
    assert_eq!(
        sys_linux_faccessat(LINUX_AT_FDCWD, 0, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn faccessat_rejects_unknown_flags() {
    let path = b"/tmp\0";
    assert_eq!(
        sys_linux_faccessat(LINUX_AT_FDCWD, path.as_ptr() as usize, 0, 0x8000),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn faccessat_empty_path_without_fd_returns_ebadf() {
    #[cfg(feature = "posix_fs")]
    {
        let empty = b"\0";
        assert_eq!(
            sys_linux_faccessat(-2, empty.as_ptr() as usize, 0, LINUX_AT_EMPTY_PATH),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }
}

#[test_case]
fn faccessat_empty_path_uses_dirfd_context() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_faccessat_empty")
            .expect("mount");
        let fd = crate::modules::posix::fs::open(fs_id, "/visible", true).expect("open");
        let empty = b"\0";
        assert_eq!(
            sys_linux_faccessat(fd as isize, empty.as_ptr() as usize, 0, LINUX_AT_EMPTY_PATH),
            0
        );
        let _ = crate::modules::posix::fs::close(fd);
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}
