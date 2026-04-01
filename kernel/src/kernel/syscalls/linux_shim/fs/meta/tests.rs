    use super::*;

    #[test_case]
    fn newfstatat_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_newfstatat(LINUX_AT_FDCWD as usize, 0, 0x1000, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn newfstatat_rejects_unknown_flags() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_newfstatat(
                LINUX_AT_FDCWD as usize,
                path.as_ptr() as usize,
                0x1000,
                0x4000
            ),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn newfstatat_empty_path_uses_dirfd_context() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_newfstatat_empty")
                .expect("mount");
            let fd = crate::modules::posix::fs::open(fs_id, "/tracked", true).expect("open");
            let empty = b"\0";
            let mut stat_buf = [0u8; LINUX_STAT_BUF_LEN];
            assert_eq!(
                sys_linux_newfstatat(
                    fd as usize,
                    empty.as_ptr() as usize,
                    stat_buf.as_mut_ptr() as usize,
                    LINUX_AT_EMPTY_PATH,
                ),
                0
            );
            let _ = crate::modules::posix::fs::close(fd);
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn statx_rejects_unknown_flags() {
        let path = b"/tmp\0";
        let mut out = [0u8; 256];
        assert_eq!(
            sys_linux_statx(
                LINUX_AT_FDCWD as usize,
                path.as_ptr() as usize,
                0x8000,
                crate::kernel::syscalls::syscalls_consts::linux::STATX_BASIC_STATS as usize,
                out.as_mut_ptr() as usize,
            ),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn statx_empty_path_uses_open_fd_context() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_statx_empty")
                .expect("mount");
            let fd = crate::modules::posix::fs::open(fs_id, "/tracked", true).expect("open");
            let empty = b"\0";
            let mut out = [0u8; 256];
            assert_eq!(
                sys_linux_statx(
                    fd as usize,
                    empty.as_ptr() as usize,
                    LINUX_AT_EMPTY_PATH,
                    crate::kernel::syscalls::syscalls_consts::linux::STATX_BASIC_STATS as usize,
                    out.as_mut_ptr() as usize,
                ),
                0
            );
            let _ = crate::modules::posix::fs::close(fd);
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn statfs_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_statfs(0, 0x2000),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn fstatfs_invalid_buffer_pointer_returns_efault_or_ebadf() {
        let rc = sys_linux_fstatfs(usize::MAX, 0x1);
        assert!(
            rc == linux_errno(crate::modules::posix_consts::errno::EBADF)
                || rc == linux_errno(crate::modules::posix_consts::errno::ENOSYS)
        );
    }

    #[test_case]
    fn statfs_successfully_writes_struct() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id =
                crate::modules::posix::fs::mount_ramfs("/linux_shim_statfs").expect("mount");
            let mut out = [0u8; core::mem::size_of::<LinuxStatfs>()];
            let path = b"/linux_shim_statfs\0";
            assert_eq!(
                sys_linux_statfs(path.as_ptr() as usize, out.as_mut_ptr() as usize),
                0
            );
            let written = unsafe { &*(out.as_ptr() as *const LinuxStatfs) };
            assert_eq!(written.f_fsid[0], fs_id as i32);
            assert!(written.f_bsize > 0);
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn utimensat_invalid_times_pointer_returns_efault() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id =
                crate::modules::posix::fs::mount_ramfs("/linux_shim_utimens").expect("mount");
            let file = crate::modules::posix::fs::open(fs_id, "/stamp", true).expect("open");
            let _ = crate::modules::posix::fs::close(file);
            let path = b"/stamp\0";
            assert_eq!(
                sys_linux_utimensat(LINUX_AT_FDCWD as usize, path.as_ptr() as usize, 0x1, 0),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn utimensat_rejects_unknown_flags() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_utimensat(LINUX_AT_FDCWD as usize, path.as_ptr() as usize, 0, 0x4000),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn utimensat_rejects_negative_nsec() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id = crate::modules::posix::fs::mount_ramfs("/linux_shim_utimens_negative_nsec")
                .expect("mount");
            let file = crate::modules::posix::fs::open(fs_id, "/stamp", true).expect("open");
            let _ = crate::modules::posix::fs::close(file);
            let path = b"/stamp\0";
            let times = [
                LinuxTimespec {
                    tv_sec: 1,
                    tv_nsec: -1,
                },
                LinuxTimespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
            ];
            assert_eq!(
                sys_linux_utimensat(
                    LINUX_AT_FDCWD as usize,
                    path.as_ptr() as usize,
                    times.as_ptr() as usize,
                    0,
                ),
                linux_errno(crate::modules::posix_consts::errno::EINVAL)
            );
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn utimensat_empty_path_uses_dirfd_context() {
        #[cfg(feature = "posix_fs")]
        {
            let fs_id =
                crate::modules::posix::fs::mount_ramfs("/linux_shim_utimens_empty").expect("mount");
            let fd = crate::modules::posix::fs::open(fs_id, "/stamp", true).expect("open");
            let empty = b"\0";
            assert_eq!(
                sys_linux_utimensat(fd as usize, empty.as_ptr() as usize, 0, LINUX_AT_EMPTY_PATH,),
                0
            );
            let _ = crate::modules::posix::fs::close(fd);
            let _ = crate::modules::posix::fs::unmount(fs_id);
        }
    }

    #[test_case]
    fn chmod_invalid_path_pointer_returns_efault() {
        assert_eq!(
            sys_linux_chmod(0, 0o644),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
