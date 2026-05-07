use super::*;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;
#[cfg(feature = "posix_fs")]
use crate::kernel::syscalls::linux_shim::fs::sys_linux_close;

#[inline]
fn openat2_with_resolve(path: &[u8], resolve: u64) -> usize {
    let how = LinuxOpenHowCompat {
        flags: 0,
        mode: 0,
        resolve,
    };
    sys_linux_openat2(
        LINUX_AT_FDCWD,
        path.as_ptr() as usize,
        (&how as *const LinuxOpenHowCompat) as usize,
        core::mem::size_of::<LinuxOpenHowCompat>(),
    )
}

#[test_case]
fn openat_invalid_path_pointer_returns_efault() {
    assert_eq!(
        sys_linux_openat(LINUX_AT_FDCWD, 0, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn openat_invalid_dirfd_returns_ebadf_for_relative_paths() {
    let path = b"relative\0";
    assert_eq!(
        sys_linux_openat(-2, path.as_ptr() as usize, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    );
}

#[test_case]
fn openat_creat_excl_returns_eexist_when_target_exists() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id =
            crate::modules::posix::fs::mount_ramfs("/linux_shim_openat_creat_excl").expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_openat_creat_excl/pkg", true)
            .expect("create existing");

        let path = b"/linux_shim_openat_creat_excl/pkg\0";
        assert_eq!(
            sys_linux_openat(LINUX_AT_FDCWD, path.as_ptr() as usize, LINUX_O_CREAT | LINUX_O_EXCL, 0o644),
            linux_errno(crate::modules::posix_consts::errno::EEXIST)
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn openat_creat_excl_creates_when_target_absent() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id =
            crate::modules::posix::fs::mount_ramfs("/linux_shim_openat_creat_excl_new").expect("mount fs");
        let path = b"/linux_shim_openat_creat_excl_new/pkg\0";

        let fd = sys_linux_openat(
            LINUX_AT_FDCWD,
            path.as_ptr() as usize,
            LINUX_O_CREAT | LINUX_O_EXCL,
            0o644,
        );
        assert!(fd <= u32::MAX as usize);
        assert_eq!(sys_linux_close(fd), 0);

        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_openat_creat_excl_new/pkg")
                .unwrap_or(false),
            true
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn openat2_resolve_cached_returns_eagain() {
    let path = b"/tmp\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_CACHED as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::EAGAIN)
    );
}

#[test_case]
fn openat2_resolve_beneath_rejects_absolute_path() {
    let path = b"/abs\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_BENEATH as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::EXDEV)
    );
}

#[test_case]
fn openat2_resolve_in_root_rejects_parent_traversal() {
    let path = b"a/../b\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_IN_ROOT as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::EXDEV)
    );
}

#[test_case]
fn openat2_resolve_no_symlinks_rejects_symlink_target() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_openat2_no_symlink")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_openat2_no_symlink/real", true)
            .expect("create real file");
        crate::modules::posix::fs::symlink(
            fs_id,
            "/linux_shim_openat2_no_symlink/real",
            "/linux_shim_openat2_no_symlink/link",
        )
        .expect("create symlink");

        let path = b"/linux_shim_openat2_no_symlink/link\0";
        assert_eq!(
            openat2_with_resolve(
                path,
                crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_SYMLINKS
                    as u64,
            ),
            linux_errno(crate::modules::posix_consts::errno::ELOOP)
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn openat2_resolve_no_symlinks_rejects_intermediate_symlink() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_openat2_no_symlink_mid")
            .expect("mount fs");
        crate::modules::posix::fs::mkdir(fs_id, "/linux_shim_openat2_no_symlink_mid/real", 0o755)
            .expect("mkdir real");
        let _ = crate::modules::posix::fs::open(
            fs_id,
            "/linux_shim_openat2_no_symlink_mid/real/file",
            true,
        )
        .expect("create real file");

        crate::modules::posix::fs::symlink(
            fs_id,
            "/linux_shim_openat2_no_symlink_mid/real",
            "/linux_shim_openat2_no_symlink_mid/linkdir",
        )
        .expect("create intermediate symlink");

        let path = b"/linux_shim_openat2_no_symlink_mid/linkdir/file\0";
        assert_eq!(
            openat2_with_resolve(
                path,
                crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_SYMLINKS
                    as u64,
            ),
            linux_errno(crate::modules::posix_consts::errno::ELOOP)
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn openat2_resolve_no_xdev_rejects_absolute_path() {
    let path = b"/outside\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_XDEV as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::EXDEV)
    );
}

#[test_case]
fn openat2_resolve_no_magiclinks_rejects_proc_fd_path() {
    let path = b"/proc/self/fd/1\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_MAGICLINKS
                as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::ELOOP)
    );
}

#[test_case]
fn openat2_resolve_no_magiclinks_rejects_proc_exe_path() {
    let path = b"/proc/self/exe\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_MAGICLINKS
                as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::ELOOP)
    );
}

#[test_case]
fn openat2_resolve_no_magiclinks_rejects_proc_thread_self_fd_path() {
    let path = b"/proc/thread-self/fd/1\0";
    assert_eq!(
        openat2_with_resolve(
            path,
            crate::kernel::syscalls::syscalls_consts::linux::openat2::RESOLVE_NO_MAGICLINKS
                as u64,
        ),
        linux_errno(crate::modules::posix_consts::errno::ELOOP)
    );
}
