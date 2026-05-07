use super::super::super::*;
use crate::kernel::syscalls::linux_shim::fs::support::{resolve_path_at_allow_empty, LINUX_AT_EMPTY_PATH, LINUX_AT_SYMLINK_NOFOLLOW};

#[repr(C)]
#[cfg(feature = "posix_fs")]
#[derive(Clone, Copy)]
pub(crate) struct LinuxTimespec {
    pub(crate) tv_sec: i64,
    pub(crate) tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_utimensat(
    dirfd: usize,
    pathname_ptr: usize,
    times_ptr: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if (flags & !(LINUX_AT_SYMLINK_NOFOLLOW | LINUX_AT_EMPTY_PATH)) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let allow_empty = (flags & LINUX_AT_EMPTY_PATH) != 0;
        let (fs_id, resolved) =
            match resolve_path_at_allow_empty(dirfd as isize, pathname_ptr, allow_empty) {
                Ok(v) => v,
                Err(err) => return err,
            };
        if times_ptr == 0 {
            return match crate::modules::posix::fs::utimensat(fs_id, &resolved) {
                Ok(()) => 0,
                Err(err) => linux_errno(err.code()),
            };
        }

        let times = with_user_read_bytes(
            times_ptr,
            core::mem::size_of::<LinuxTimespec>() * 2,
            |src| {
                let mut buf = [0u8; core::mem::size_of::<LinuxTimespec>() * 2];
                buf.copy_from_slice(src);
                let mut at_raw = [0u8; core::mem::size_of::<LinuxTimespec>()];
                let mut mt_raw = [0u8; core::mem::size_of::<LinuxTimespec>()];
                at_raw.copy_from_slice(&buf[..core::mem::size_of::<LinuxTimespec>()]);
                mt_raw.copy_from_slice(&buf[core::mem::size_of::<LinuxTimespec>()..]);
                let at =
                    unsafe { core::ptr::read_unaligned(at_raw.as_ptr() as *const LinuxTimespec) };
                let mt =
                    unsafe { core::ptr::read_unaligned(mt_raw.as_ptr() as *const LinuxTimespec) };
                (at, mt)
            },
        )
        .unwrap_or_else(|_| {
            (
                LinuxTimespec {
                    tv_sec: i64::MIN,
                    tv_nsec: i64::MIN,
                },
                LinuxTimespec {
                    tv_sec: i64::MIN,
                    tv_nsec: i64::MIN,
                },
            )
        });
        if times.0.tv_sec == i64::MIN && times.0.tv_nsec == i64::MIN {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }
        if !(0..1_000_000_000).contains(&times.0.tv_nsec)
            || !(0..1_000_000_000).contains(&times.1.tv_nsec)
        {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let atime = crate::modules::posix::time::PosixTimespec {
            sec: times.0.tv_sec,
            nsec: times.0.tv_nsec as i32,
        };
        let mtime = crate::modules::posix::time::PosixTimespec {
            sec: times.1.tv_sec,
            nsec: times.1.tv_nsec as i32,
        };
        match crate::modules::posix::fs::utimes(fs_id, &resolved, atime, mtime) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, pathname_ptr, times_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_futimesat(dirfd: usize, pathname_ptr: usize, times_ptr: usize) -> usize {
    sys_linux_utimensat(dirfd, pathname_ptr, times_ptr, 0)
}
