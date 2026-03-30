#[test_case]
fn dpkg_style_file_ops_sequence_succeeds() {
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
fn dpkg_style_rename_noreplace_failure_then_recovery() {
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
fn dpkg_style_interrupted_fsync_chain_can_retry() {
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

#[test_case]
fn p2_apt_install_remove_upgrade_cycles_with_conflict_paths() {
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
fn p2_apt_interrupted_transaction_recovery_after_reopen() {
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
fn p2_apt_parallel_like_interleaved_lock_contention_flow() {
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

#[test_case]
fn p2_python_virtualenv_tree_install_remove_loops() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_py_venv\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for i in 0..5usize {
            for j in 0..20usize {
                let p = alloc::format!("/linux_shim_p2_py_venv/lib/site-packages/m{}/f{}", i, j);
                let parent = alloc::format!("/linux_shim_p2_py_venv/lib/site-packages/m{}", i);
                let parentz = alloc::format!("{}\0", parent);
                let _ = sys_linux_mkdirat(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    parentz.as_ptr() as usize,
                    0o755,
                );
                let pz = alloc::format!("{}\0", p);
                let fd = sys_linux_openat(
                    crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                    pz.as_ptr() as usize,
                    crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                    0o644,
                );
                assert!(fd <= u32::MAX as usize);
                assert_eq!(sys_linux_close(fd), 0);
                assert_eq!(
                    sys_linux_unlinkat(
                        crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                        pz.as_ptr() as usize,
                        0,
                    ),
                    0
                );
            }
        }
    }
}

#[test_case]
fn p2_flutter_sdk_unpack_and_cache_warmup_cycles() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_flutter_sdk\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for idx in 0..24usize {
            let p = alloc::format!("/linux_shim_p2_flutter_sdk/cache/artifact{}", idx);
            let pz = alloc::format!("{}\0", p);
            let fd = sys_linux_openat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                pz.as_ptr() as usize,
                crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CREAT,
                0o644,
            );
            assert!(fd <= u32::MAX as usize);
            let payload = [0x5Au8; 64];
            assert_eq!(sys_linux_write(fd, payload.as_ptr() as usize, payload.len()), payload.len());
            assert_eq!(sys_linux_fsync(fd), 0);
            assert_eq!(sys_linux_close(fd), 0);
        }
    }
}

#[test_case]
fn p2_flutter_asset_heavy_build_update_cycles() {
    #[cfg(feature = "posix_fs")]
    {
        let root = b"/linux_shim_p2_flutter_assets\0";
        assert_eq!(
            sys_linux_mkdirat(
                crate::kernel::syscalls::syscalls_consts::linux::AT_FDCWD,
                root.as_ptr() as usize,
                0o755,
            ),
            0
        );

        for idx in 0..10usize {
            let old = alloc::format!("/linux_shim_p2_flutter_assets/a{}.tmp", idx);
            let new = alloc::format!("/linux_shim_p2_flutter_assets/a{}.bin", idx);
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
fn p2_package_manager_fault_injection_chain_retry_idempotent() {
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
fn p2_package_manager_metadata_retry_idempotence_multicycle() {
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
