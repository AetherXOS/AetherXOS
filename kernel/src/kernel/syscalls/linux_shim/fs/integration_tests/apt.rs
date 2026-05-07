#[cfg(feature = "posix_fs")]
use super::*;

#[test_case]
pub fn p2_apt_install_remove_upgrade_cycles_with_conflict_paths() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_apt_cycles\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for idx in 0..8usize {
            let old = alloc::format!("/linux_shim_p2_apt_cycles/pkg{}.old", idx);
            let new = alloc::format!("/linux_shim_p2_apt_cycles/pkg{}.new", idx);
            let oldz = alloc::format!("{}\0", old);
            let newz = alloc::format!("{}\0", new);

            let fd = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                oldz.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                0o644,
            );
            assert!(fd <= u32::MAX as usize);
            assert_eq!(sys_linux_close(fd), 0);

            let conflict = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                newz.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                0o644,
            );
            assert!(conflict <= u32::MAX as usize);
            assert_eq!(sys_linux_close(conflict), 0);

            assert_eq!(
                sys_linux_renameat2(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    oldz.as_ptr() as usize,
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    newz.as_ptr() as usize,
                    1,
                ),
                crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EEXIST)
            );

            assert_eq!(
                sys_linux_unlinkat(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    newz.as_ptr() as usize,
                    0,
                ),
                0
            );
            assert_eq!(
                sys_linux_renameat2(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    oldz.as_ptr() as usize,
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    newz.as_ptr() as usize,
                    1,
                ),
                0
            );
        }
    }
}

#[test_case]
pub fn p2_apt_interrupted_transaction_recovery_after_reopen() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_apt_recovery\0";
        let path = b"/linux_shim_p2_apt_recovery/db\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        let fd = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            path.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(fd <= u32::MAX as usize);
        let payload = b"txn-step";
        assert_eq!(sys_linux_write(fd, payload.as_ptr() as usize, payload.len()), payload.len());
        assert_eq!(sys_linux_close(fd), 0);

        assert_eq!(
            sys_linux_fsync(fd),
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EBADF)
        );

        let fd2 = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            path.as_ptr() as usize,
            0,
            0,
        );
        assert!(fd2 <= u32::MAX as usize);
        assert_eq!(sys_linux_fsync(fd2), 0);
        assert_eq!(sys_linux_close(fd2), 0);
    }
}

#[test_case]
pub fn p2_apt_parallel_like_interleaved_lock_contention_flow() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_apt_parallel\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for idx in 0..16usize {
            let p = alloc::format!("/linux_shim_p2_apt_parallel/f{}", idx);
            let pz = alloc::format!("{}\0", p);
            let fd = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                pz.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                0o644,
            );
            assert!(fd <= u32::MAX as usize);
            let bytes = [idx as u8; 8];
            assert_eq!(sys_linux_write(fd, bytes.as_ptr() as usize, bytes.len()), bytes.len());
            assert_eq!(sys_linux_fdatasync(fd), 0);
            assert_eq!(sys_linux_close(fd), 0);
        }
    }
}
