use super::*;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;

#[test_case]
fn mkdirat_invalid_path_pointer_returns_efault() {
    assert_eq!(
        sys_linux_mkdirat(LINUX_AT_FDCWD, 0, 0o755),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn unlinkat_invalid_path_pointer_returns_efault() {
    assert_eq!(
        sys_linux_unlinkat(LINUX_AT_FDCWD, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn linkat_invalid_oldpath_pointer_returns_efault() {
    let newp = b"/tmp_new\0";
    assert_eq!(
        sys_linux_linkat(
            LINUX_AT_FDCWD,
            0,
            LINUX_AT_FDCWD,
            newp.as_ptr() as usize,
            0,
        ),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn symlinkat_invalid_target_pointer_returns_efault() {
    let link = b"/tmp_link\0";
    assert_eq!(
        sys_linux_symlinkat(0, LINUX_AT_FDCWD, link.as_ptr() as usize),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn symlinkat_invalid_linkpath_pointer_returns_efault() {
    let target = b"/tmp_target\0";
    assert_eq!(
        sys_linux_symlinkat(target.as_ptr() as usize, LINUX_AT_FDCWD, 0),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn linkat_rejects_unknown_flags() {
    let oldp = b"/old\0";
    let newp = b"/new\0";
    assert_eq!(
        sys_linux_linkat(
            LINUX_AT_FDCWD,
            oldp.as_ptr() as usize,
            LINUX_AT_FDCWD,
            newp.as_ptr() as usize,
            0x8000,
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn linkat_cross_filesystem_returns_exdev() {
    #[cfg(feature = "posix_fs")]
    {
        let old_fs =
            crate::modules::posix::fs::mount_ramfs("/linux_shim_linkat_old").expect("mount old fs");
        let new_fs =
            crate::modules::posix::fs::mount_ramfs("/linux_shim_linkat_new").expect("mount new fs");

        let _ = crate::modules::posix::fs::open(old_fs, "/linux_shim_linkat_old/file", true)
            .expect("create old file");

        let oldp = b"/linux_shim_linkat_old/file\0";
        let newp = b"/linux_shim_linkat_new/file_link\0";
        assert_eq!(
            sys_linux_linkat(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                0,
            ),
            linux_errno(crate::modules::posix_consts::errno::EXDEV)
        );

        let _ = crate::modules::posix::fs::unmount(old_fs);
        let _ = crate::modules::posix::fs::unmount(new_fs);
    }
}
