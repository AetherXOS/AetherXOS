use super::*;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD;

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
