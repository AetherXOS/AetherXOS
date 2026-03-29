#[cfg(feature = "posix_fs")]
use super::super::util::read_user_c_string_allow_empty;
use super::super::*;
#[cfg(feature = "posix_fs")]
use super::support::{
    resolve_path_at_allow_empty, validate_newfstatat_flags, LINUX_AT_EMPTY_PATH,
    LINUX_AT_SYMLINK_NOFOLLOW,
};
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_STAT_BUF_LEN: usize = 144;
#[cfg(not(feature = "linux_compat"))]
mod statx;

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
#[derive(Clone, Copy)]
struct LinuxTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
#[derive(Clone, Copy)]
struct LinuxStatfs {
    f_type: i64,
    f_bsize: i64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_fsid: [i32; 2],
    f_namelen: i64,
    f_frsize: i64,
    f_flags: i64,
    f_spare: [i64; 4],
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn fill_linux_statfs(fs_id: u32, stats: crate::modules::posix::fs::PosixFsStats) -> LinuxStatfs {
    LinuxStatfs {
        f_type: if fs_id == 1 {
            crate::kernel::syscalls::syscalls_consts::linux::RAMFS_MAGIC as i64
        } else {
            0xbeef_fadeu32 as i64
        },
        f_bsize: stats.f_bsize as i64,
        f_blocks: stats.f_blocks,
        f_bfree: stats.f_bfree,
        f_bavail: stats.f_bavail,
        f_files: stats.f_files,
        f_ffree: stats.f_ffree,
        f_fsid: [fs_id as i32, 0],
        f_namelen: stats.f_namelen as i64,
        f_frsize: stats.f_bsize as i64,
        f_flags: 0,
        f_spare: [0; 4],
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn write_linux_statfs(buf_ptr: usize, stats: LinuxStatfs) -> usize {
    with_user_write_bytes(buf_ptr, core::mem::size_of::<LinuxStatfs>(), |dst| {
        let src_ptr = &stats as *const LinuxStatfs as *const u8;
        let src =
            unsafe { core::slice::from_raw_parts(src_ptr, core::mem::size_of::<LinuxStatfs>()) };
        dst.copy_from_slice(src);
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fstat(fd: usize, buf_ptr: usize) -> usize {
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
pub(super) fn sys_linux_newfstatat(
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

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statx(
    dirfd: usize,
    pathname_ptr: usize,
    flags: usize,
    mask: usize,
    statxbuf_ptr: usize,
) -> usize {
    statx::sys_linux_statx(dirfd, pathname_ptr, flags, mask, statxbuf_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_ftruncate(fd: usize, length: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::ftruncate(fd as u32, length) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, length);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fsync(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fsync(fd as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fdatasync(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fdatasync(fd as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_chmod(path_ptr: usize, mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, mode);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchmod(fd: usize, mode: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fchmod(fd as u32, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, mode);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_chown(path_ptr: usize, uid: usize, gid: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chown(fs_id, &resolved, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, uid, gid);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchown(fd: usize, uid: usize, gid: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::fchown(fd as u32, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, uid, gid);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchmodat(
    dirfd: usize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if (flags & !LINUX_AT_SYMLINK_NOFOLLOW) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let (fs_id, resolved) = match resolve_path_at_allow_empty(dirfd as isize, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, mode, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchownat(
    dirfd: usize,
    path_ptr: usize,
    uid: usize,
    gid: usize,
    flags: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if (flags & !LINUX_AT_SYMLINK_NOFOLLOW) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        let (fs_id, resolved) = match resolve_path_at_allow_empty(dirfd as isize, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::fs::chown(fs_id, &resolved, uid as u32, gid as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, path_ptr, uid, gid, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statfs(path_ptr: usize, buf_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (fs_id, resolved) = match resolve_path_at_allow_empty(LINUX_AT_FDCWD, path_ptr, false) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if let Err(err) = crate::modules::posix::fs::stat(fs_id, &resolved) {
            return linux_errno(err.code());
        }
        let stats = match crate::modules::posix::fs::statfs(fs_id) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_linux_statfs(buf_ptr, fill_linux_statfs(fs_id, stats))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (path_ptr, buf_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fstatfs(fd: usize, buf_ptr: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd as u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let stats = match crate::modules::posix::fs::statfs(fs_id) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_linux_statfs(buf_ptr, fill_linux_statfs(fs_id, stats))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, buf_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_utimensat(
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
pub(super) fn sys_linux_futimesat(dirfd: usize, pathname_ptr: usize, times_ptr: usize) -> usize {
    sys_linux_utimensat(dirfd, pathname_ptr, times_ptr, 0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sync() -> usize {
    #[cfg(feature = "posix_fs")]
    {
        if let Ok(fs_id) = crate::modules::posix::fs::default_fs_id() {
            let _ = crate::modules::posix::fs::syncfs(fs_id);
        }
        0
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_syncfs(fd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd as u32) {
            Ok(id) => id,
            Err(err) => return linux_errno(err.code()),
        };
        match crate::modules::posix::fs::syncfs(fs_id) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = fd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "meta/tests.rs"]
mod tests;
