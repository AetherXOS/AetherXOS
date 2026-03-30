use super::super::super::*;
#[cfg(feature = "posix_fs")]
use super::super::support::{resolve_path_at_allow_empty, LINUX_AT_EMPTY_PATH, LINUX_AT_NO_AUTOMOUNT, LINUX_AT_SYMLINK_NOFOLLOW};
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use crate::kernel::syscalls::linux_shim::util::write_user_pod;

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
#[derive(Clone, Copy, Default)]
struct LinuxStatxTimestampCompat {
    tv_sec: i64,
    tv_nsec: u32,
    __reserved: i32,
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
#[derive(Clone, Copy, Default)]
struct LinuxStatxCompat {
    stx_mask: u32,
    stx_blksize: u32,
    stx_attributes: u64,
    stx_nlink: u32,
    stx_uid: u32,
    stx_gid: u32,
    stx_mode: u16,
    __spare0: u16,
    stx_ino: u64,
    stx_size: u64,
    stx_blocks: u64,
    stx_attributes_mask: u64,
    stx_atime: LinuxStatxTimestampCompat,
    stx_btime: LinuxStatxTimestampCompat,
    stx_ctime: LinuxStatxTimestampCompat,
    stx_mtime: LinuxStatxTimestampCompat,
    stx_rdev_major: u32,
    stx_rdev_minor: u32,
    stx_dev_major: u32,
    stx_dev_minor: u32,
    stx_mnt_id: u64,
    __spare2: [u64; 13],
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
fn write_linux_statx(mask: usize, statxbuf_ptr: usize, st: crate::modules::posix::fs::PosixStat) -> usize {
    let mut stx: LinuxStatxCompat = LinuxStatxCompat::default();
    stx.stx_mask = crate::kernel::syscalls::syscalls_consts::linux::STATX_BASIC_STATS & mask as u32;
    stx.stx_blksize = crate::kernel::syscalls::syscalls_consts::linux::STAT_BLKSIZE as u32;
    stx.stx_nlink = 1;
    stx.stx_uid = st.uid;
    stx.stx_gid = st.gid;
    stx.stx_mode = st.mode;
    if st.is_dir {
        stx.stx_mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFDIR as u16;
    } else if st.is_symlink {
        stx.stx_mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFLNK as u16;
    } else {
        stx.stx_mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFREG as u16;
    }
    stx.stx_ino = st.ino;
    stx.stx_size = st.size;
    stx.stx_blocks = (st.size
        + (crate::kernel::syscalls::syscalls_consts::linux::STAT_BLOCK_SIZE - 1) as u64)
        / crate::kernel::syscalls::syscalls_consts::linux::STAT_BLOCK_SIZE as u64;
    stx.stx_atime = LinuxStatxTimestampCompat {
        tv_sec: st.atime,
        tv_nsec: 0,
        __reserved: 0,
    };
    stx.stx_mtime = LinuxStatxTimestampCompat {
        tv_sec: st.mtime,
        tv_nsec: 0,
        __reserved: 0,
    };
    stx.stx_ctime = LinuxStatxTimestampCompat {
        tv_sec: st.ctime,
        tv_nsec: 0,
        __reserved: 0,
    };
    stx.stx_btime = stx.stx_ctime;

    write_user_pod(statxbuf_ptr, &stx)
        .map(|_| 0usize)
        .unwrap_or_else(|err| err)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statx(
    dirfd: usize,
    pathname_ptr: usize,
    flags: usize,
    mask: usize,
    statxbuf_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        const LINUX_STATX_SYNC_TYPE: usize = 0x6000;
        let allowed_flags =
            LINUX_AT_EMPTY_PATH | LINUX_AT_SYMLINK_NOFOLLOW | LINUX_AT_NO_AUTOMOUNT | LINUX_STATX_SYNC_TYPE;
        if (flags & !allowed_flags) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        if statxbuf_ptr == 0 {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }

        let allow_empty = (flags & LINUX_AT_EMPTY_PATH) != 0;
        if allow_empty && pathname_ptr == 0 {
            return match crate::modules::posix::fs::fstat(dirfd as u32) {
                Ok(st) => write_linux_statx(mask, statxbuf_ptr, st),
                Err(err) => linux_errno(err.code()),
            };
        }

        let (fs_id, resolved) =
            match resolve_path_at_allow_empty(dirfd as isize, pathname_ptr, allow_empty) {
                Ok(v) => v,
                Err(err) => return err,
            };
        let fd = match crate::modules::posix::fs::open(fs_id, &resolved, false) {
            Ok(fd) => fd,
            Err(err) => return linux_errno(err.code()),
        };
        let out = match crate::modules::posix::fs::fstat(fd) {
            Ok(st) => write_linux_statx(mask, statxbuf_ptr, st),
            Err(err) => linux_errno(err.code()),
        };
        let _ = crate::modules::posix::fs::close(fd);
        out
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (dirfd, pathname_ptr, flags, mask, statxbuf_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}
