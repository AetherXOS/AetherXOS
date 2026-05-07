use crate::kernel::syscalls::{linux_errno, with_user_write_bytes};
use crate::kernel::syscalls::linux_shim::fs::support::resolve_path_at;

pub(crate) fn sys_linux_readlinkat(
    dirfd: isize,
    path_ptr: usize,
    buf_ptr: usize,
    buf_size: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at(dirfd, path_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let target = match crate::modules::posix::fs::readlink(fs_id, &resolved) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let out = target.as_bytes();
        let copy_len = core::cmp::min(buf_size, out.len());
        with_user_write_bytes(buf_ptr, copy_len, |dst| {
            dst.copy_from_slice(&out[..copy_len]);
            copy_len
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, buf_ptr, buf_size);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}
