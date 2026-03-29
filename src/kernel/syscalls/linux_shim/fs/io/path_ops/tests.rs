use super::*;
#[cfg(feature = "posix_fs")]
use crate::kernel::syscalls::linux_shim::fs::support::LINUX_AT_EMPTY_PATH;

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

#[test_case]
fn mkdirat_invalid_path_pointer_returns_efault() {
    assert_eq!(
        sys_linux_mkdirat(LINUX_AT_FDCWD, 0, 0o755),
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

#[test_case]
fn renameat_invalid_path_pointer_returns_efault() {
    let path = b"/tmp\0";
    assert_eq!(
        sys_linux_renameat(LINUX_AT_FDCWD, 0, LINUX_AT_FDCWD, path.as_ptr() as usize,),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
}

#[test_case]
fn readlinkat_invalid_buffer_pointer_returns_efault() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_readlink").expect("mount");
        let _ = crate::modules::posix::fs::symlink(fs_id, "/target", "/ln");
        let path = b"/ln\0";
        assert_eq!(
            sys_linux_readlinkat(LINUX_AT_FDCWD, path.as_ptr() as usize, 0x1, 8,),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn readlinkat_truncates_to_caller_buffer_length() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_readlink_truncate")
            .expect("mount");
        let _ = crate::modules::posix::fs::symlink(fs_id, "/long-target", "/ln");
        let path = b"/ln\0";
        let mut buf = [0u8; 4];
        assert_eq!(
            sys_linux_readlinkat(
                LINUX_AT_FDCWD,
                path.as_ptr() as usize,
                buf.as_mut_ptr() as usize,
                buf.len(),
            ),
            buf.len()
        );
        assert_eq!(&buf, b"/lon");
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_rejects_unknown_flags() {
    let oldp = b"/old\0";
    let newp = b"/new\0";
    assert_eq!(
        sys_linux_renameat2(
            LINUX_AT_FDCWD,
            oldp.as_ptr() as usize,
            LINUX_AT_FDCWD,
            newp.as_ptr() as usize,
            0x80,
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn renameat2_rejects_noreplace_exchange_combination() {
    let oldp = b"/old\0";
    let newp = b"/new\0";
    assert_eq!(
        sys_linux_renameat2(
            LINUX_AT_FDCWD,
            oldp.as_ptr() as usize,
            LINUX_AT_FDCWD,
            newp.as_ptr() as usize,
            1 | 2,
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn renameat2_noreplace_returns_eexist_when_target_exists() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_eexist")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_eexist/old", true)
            .expect("create old");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_eexist/new", true)
            .expect("create new");

        let oldp = b"/linux_shim_renameat2_eexist/old\0";
        let newp = b"/linux_shim_renameat2_eexist/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                1,
            ),
            linux_errno(crate::modules::posix_consts::errno::EEXIST)
        );
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_noreplace_renames_when_target_absent() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_ok")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_ok/old", true)
            .expect("create old");

        let oldp = b"/linux_shim_renameat2_ok/old\0";
        let newp = b"/linux_shim_renameat2_ok/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                1,
            ),
            0
        );
        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_renameat2_ok/new").unwrap_or(false),
            true
        );
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_exchange_requires_both_paths() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_exchange_missing")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_exchange_missing/old", true)
            .expect("create old");

        let oldp = b"/linux_shim_renameat2_exchange_missing/old\0";
        let newp = b"/linux_shim_renameat2_exchange_missing/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                2,
            ),
            linux_errno(crate::modules::posix_consts::errno::ENOENT)
        );
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_exchange_swaps_paths() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_exchange_ok")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_exchange_ok/old", true)
            .expect("create old");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_exchange_ok/new", true)
            .expect("create new");

        let oldp = b"/linux_shim_renameat2_exchange_ok/old\0";
        let newp = b"/linux_shim_renameat2_exchange_ok/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                2,
            ),
            0
        );

        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_renameat2_exchange_ok/old")
                .unwrap_or(false),
            true
        );
        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_renameat2_exchange_ok/new")
                .unwrap_or(false),
            true
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_exchange_returns_eagain_when_tmp_namespace_exhausted_and_preserves_paths() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_exchange_eagain")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(
            fs_id,
            "/linux_shim_renameat2_exchange_eagain/old",
            true,
        )
        .expect("create old");
        let _ = crate::modules::posix::fs::open(
            fs_id,
            "/linux_shim_renameat2_exchange_eagain/new",
            true,
        )
        .expect("create new");

        for idx in 0..16u8 {
            let suffix = if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + (idx - 10)) as char
            };
            let p = alloc::format!(
                "/linux_shim_renameat2_exchange_eagain/new.hc_swap_tmp_{}",
                suffix
            );
            let _ = crate::modules::posix::fs::open(fs_id, &p, true).expect("precreate tmp slot");
        }

        let oldp = b"/linux_shim_renameat2_exchange_eagain/old\0";
        let newp = b"/linux_shim_renameat2_exchange_eagain/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                2,
            ),
            linux_errno(crate::modules::posix_consts::errno::EAGAIN)
        );

        assert_eq!(
            crate::modules::posix::fs::access(
                fs_id,
                "/linux_shim_renameat2_exchange_eagain/old",
            )
            .unwrap_or(false),
            true
        );
        assert_eq!(
            crate::modules::posix::fs::access(
                fs_id,
                "/linux_shim_renameat2_exchange_eagain/new",
            )
            .unwrap_or(false),
            true
        );

        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

#[test_case]
fn renameat2_whiteout_recreates_source_placeholder() {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_renameat2_whiteout")
            .expect("mount fs");
        let _ = crate::modules::posix::fs::open(fs_id, "/linux_shim_renameat2_whiteout/old", true)
            .expect("create old");

        let oldp = b"/linux_shim_renameat2_whiteout/old\0";
        let newp = b"/linux_shim_renameat2_whiteout/new\0";
        assert_eq!(
            sys_linux_renameat2(
                LINUX_AT_FDCWD,
                oldp.as_ptr() as usize,
                LINUX_AT_FDCWD,
                newp.as_ptr() as usize,
                4,
            ),
            0
        );

        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_renameat2_whiteout/new")
                .unwrap_or(false),
            true
        );
        assert_eq!(
            crate::modules::posix::fs::access(fs_id, "/linux_shim_renameat2_whiteout/old")
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
