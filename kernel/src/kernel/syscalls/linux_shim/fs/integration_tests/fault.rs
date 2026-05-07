#[cfg(feature = "posix_fs")]
use super::*;

#[test_case]
pub fn p2_package_manager_fault_injection_chain_retry_idempotent() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_fault_chain\0";
        let src = b"/linux_shim_p2_fault_chain/pkg.tmp\0";
        let dst = b"/linux_shim_p2_fault_chain/pkg.bin\0";

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
            src.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(fd <= u32::MAX as usize);
        let payload = b"fault-chain";
        assert_eq!(sys_linux_write(fd, payload.as_ptr() as usize, payload.len()), payload.len());
        assert_eq!(sys_linux_close(fd), 0);

        assert_eq!(
            sys_linux_renameat2(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                src.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                dst.as_ptr() as usize,
                0,
            ),
            0
        );

        assert_eq!(
            sys_linux_fsync(fd),
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EBADF)
        );

        let reopened = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            dst.as_ptr() as usize,
            0,
            0,
        );
        assert!(reopened <= u32::MAX as usize);
        assert_eq!(sys_linux_fdatasync(reopened), 0);
        assert_eq!(sys_linux_close(reopened), 0);

        assert_eq!(
            sys_linux_unlinkat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                dst.as_ptr() as usize,
                0,
            ),
            0
        );

        assert_eq!(
            sys_linux_fchmodat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                dst.as_ptr() as usize,
                0o600,
                0,
            ),
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::ENOENT)
        );
        assert_eq!(
            sys_linux_fchownat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                dst.as_ptr() as usize,
                0,
                0,
                0,
            ),
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::ENOENT)
        );

        let recreated = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            dst.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(recreated <= u32::MAX as usize);
        assert_eq!(sys_linux_close(recreated), 0);

        assert_eq!(
            sys_linux_fchmodat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                dst.as_ptr() as usize,
                0o640,
                0,
            ),
            0
        );
        let chown_retry = sys_linux_fchownat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
            dst.as_ptr() as usize,
            0,
            0,
            0,
        );
        assert!(
            chown_retry == 0
                || chown_retry
                    == crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EPERM)
        );
    }
}

#[test_case]
pub fn p2_package_manager_metadata_retry_idempotence_multicycle() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_meta_retry\0";
        let path = b"/linux_shim_p2_meta_retry/pkg.db\0";

        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for _ in 0..6usize {
            let fd = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                path.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                0o644,
            );
            assert!(fd <= u32::MAX as usize);
            assert_eq!(sys_linux_close(fd), 0);

            assert_eq!(
                sys_linux_fchmodat(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                    path.as_ptr() as usize,
                    0o640,
                    0,
                ),
                0
            );

            let chown = sys_linux_fchownat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                path.as_ptr() as usize,
                0,
                0,
                0,
            );
            assert!(
                chown == 0
                    || chown
                        == crate::kernel::syscalls::linux_errno(
                            crate::modules::posix_consts::errno::EPERM,
                        )
            );

            let opened = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                path.as_ptr() as usize,
                0,
                0,
            );
            assert!(opened <= u32::MAX as usize);
            assert_eq!(sys_linux_fdatasync(opened), 0);
            assert_eq!(sys_linux_close(opened), 0);
        }
    }
}
