#[cfg(feature = "posix_fs")]
use super::*;

#[test_case]
pub fn dpkg_style_file_ops_sequence_succeeds() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_dpkg_flow\0";
        let old = b"/linux_shim_dpkg_flow/pkg.old\0";
        let new = b"/linux_shim_dpkg_flow/pkg.new\0";
        let hard = b"/linux_shim_dpkg_flow/pkg.hard\0";
        let sym = b"/linux_shim_dpkg_flow/pkg.sym\0";
        let old_target = b"/linux_shim_dpkg_flow/pkg.old\0";

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
            old.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(fd <= u32::MAX as usize);

        let payload = b"pkg-payload";
        assert_eq!(sys_linux_write(fd, payload.as_ptr() as usize, payload.len()), payload.len());
        assert_eq!(sys_linux_fdatasync(fd), 0);
        assert_eq!(sys_linux_fsync(fd), 0);
        assert_eq!(sys_linux_close(fd), 0);

        assert_eq!(
            sys_linux_linkat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                old.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                hard.as_ptr() as usize,
                0,
            ),
            0
        );

        assert_eq!(
            sys_linux_symlinkat(
                old_target.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                sym.as_ptr() as usize,
            ),
            0
        );

        assert_eq!(
            sys_linux_renameat2(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                old.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                new.as_ptr() as usize,
                1,
            ),
            0
        );

        assert_eq!(
            sys_linux_fchmodat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
                new.as_ptr() as usize,
                0o755,
                0,
            ),
            0
        );

        let chown_rc = sys_linux_fchownat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD as usize,
            new.as_ptr() as usize,
            0,
            0,
            0,
        );
        assert!(
            chown_rc == 0
                || chown_rc
                    == crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EPERM,)
        );

        let mut link_buf = [0u8; 64];
        let n = sys_linux_readlinkat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            sym.as_ptr() as usize,
            link_buf.as_mut_ptr() as usize,
            link_buf.len(),
        );
        assert!(n > 0 && n <= link_buf.len());

        let new_fd = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            new.as_ptr() as usize,
            0,
            0,
        );
        assert!(new_fd <= u32::MAX as usize);
        assert_eq!(sys_linux_fsync(new_fd), 0);
        assert_eq!(sys_linux_close(new_fd), 0);
    }
}

#[test_case]
pub fn dpkg_style_rename_noreplace_failure_then_recovery() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_dpkg_recovery\0";
        let src = b"/linux_shim_dpkg_recovery/pkg.src\0";
        let dst = b"/linux_shim_dpkg_recovery/pkg.dst\0";

        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        let src_fd = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            src.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(src_fd <= u32::MAX as usize);
        assert_eq!(sys_linux_close(src_fd), 0);

        let dst_fd = sys_linux_openat(
            crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
            dst.as_ptr() as usize,
            crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
            0o644,
        );
        assert!(dst_fd <= u32::MAX as usize);
        assert_eq!(sys_linux_close(dst_fd), 0);

        assert_eq!(
            sys_linux_renameat2(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                src.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                dst.as_ptr() as usize,
                1,
            ),
            crate::kernel::syscalls::linux_errno(crate::modules::posix_consts::errno::EEXIST)
        );

        assert_eq!(
            sys_linux_unlinkat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                dst.as_ptr() as usize,
                0,
            ),
            0
        );

        assert_eq!(
            sys_linux_renameat2(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                src.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                dst.as_ptr() as usize,
                1,
            ),
            0
        );
    }
}

#[test_case]
pub fn dpkg_style_interrupted_fsync_chain_can_retry() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_dpkg_fsync_retry\0";
        let path = b"/linux_shim_dpkg_fsync_retry/pkg\0";

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

        let payload = b"retry";
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
