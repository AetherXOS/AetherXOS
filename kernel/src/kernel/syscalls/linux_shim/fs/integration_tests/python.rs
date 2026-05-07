#[cfg(feature = "posix_fs")]
use super::*;

#[test_case]
pub fn p2_python_virtualenv_tree_install_remove_loops() {
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
