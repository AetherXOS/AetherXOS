use super::*;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;

#[test_case]
fn renameat_invalid_path_pointer_returns_efault() {
    let path = b"/tmp\0";
    assert_eq!(
        sys_linux_renameat(LINUX_AT_FDCWD, 0, LINUX_AT_FDCWD, path.as_ptr() as usize,),
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    );
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
