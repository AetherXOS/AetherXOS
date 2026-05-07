use super::super::super::util::read_user_c_string_allow_empty;
use super::super::super::*;
use crate::kernel::syscalls::linux_shim::fs::support::{
    resolve_path_at_allow_empty, validate_newfstatat_flags, LINUX_AT_EMPTY_PATH,
};

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_STAT_BUF_LEN: usize = 144;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_fstat(fd: usize, buf_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fstat(fd as u32) {
            Ok(st) => with_user_write_bytes(buf_ptr, LINUX_STAT_BUF_LEN, |dst| {
                dst.fill(0);
                dst[48..56].copy_from_slice(&(st.size as u64).to_ne_bytes());
                dst[24..28].copy_from_slice(&(st.mode as u32).to_ne_bytes());
                dst[0..8].copy_from_slice(&(st.ino as u64).to_ne_bytes());
                dst[16..24].copy_from_slice(&(1u64).to_ne_bytes());
                dst[56..64].copy_from_slice(&(4096u64).to_ne_bytes());
                0
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT)),
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, buf_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_newfstatat(
    dirfd: usize,
    pathname_ptr: usize,
    buf_ptr: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if let Err(err) = validate_newfstatat_flags(flags) {
            return err;
        }
        let allow_empty = (flags & LINUX_AT_EMPTY_PATH) != 0;
        let (fs_id, resolved) =
            match resolve_path_at_allow_empty(dirfd as isize, pathname_ptr, allow_empty) {
                Ok(v) => v,
                Err(err) => return err,
            };

        if allow_empty && pathname_ptr != 0 {
            if let Ok(path) = read_user_c_string_allow_empty(
                pathname_ptr,
                crate::config::KernelConfig::syscall_max_path_len(),
            ) {
                if path.is_empty() {
                    return sys_linux_fstat(dirfd, buf_ptr);
                }
            }
        }

        let opened = crate::modules::posix::fs::open(fs_id, &resolved, false);

        match opened {
            Ok(fd) => {
                let result = sys_linux_fstat(fd as usize, buf_ptr);
                let _ = crate::modules::posix::fs::close(fd);
                result
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, pathname_ptr, buf_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}
