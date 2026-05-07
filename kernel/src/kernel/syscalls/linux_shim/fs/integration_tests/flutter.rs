#[cfg(feature = "posix_fs")]
use super::*;

#[test_case]
pub fn p2_flutter_sdk_unpack_and_cache_warmup_cycles() {
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
pub fn p2_flutter_asset_heavy_build_update_cycles() {
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
