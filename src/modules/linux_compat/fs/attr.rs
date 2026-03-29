use super::super::*;

/// `access(2)` — Check user's permissions for a file.
pub fn sys_linux_access(pathname_ptr: UserPtr<u8>, mode: usize) -> usize {
    sys_linux_faccessat(Fd(linux::AT_FDCWD as i32), pathname_ptr, mode, 0)
}

/// `chmod(2)` — Change permissions of a file.
pub fn sys_linux_chmod(pathname: UserPtr<u8>, mode: usize) -> usize {
    crate::require_posix_fs!((pathname, mode) => {
        syscall_path_at!(Fd(linux::AT_FDCWD as i32), pathname, write, fs_id, dir_path, path, resolved);
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `chown(2)` — Change ownership of a file.
pub fn sys_linux_chown(pathname: UserPtr<u8>, owner: usize, group: usize) -> usize {
    crate::require_posix_fs!((pathname, owner, group) => {
        syscall_path_at!(Fd(linux::AT_FDCWD as i32), pathname, write, fs_id, dir_path, path, resolved);
        match crate::modules::posix::fs::chown(fs_id, &resolved, owner as u32, group as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_lchown(pathname: UserPtr<u8>, owner: usize, group: usize) -> usize {
    sys_linux_fchownat(
        Fd(linux::AT_FDCWD as i32),
        pathname,
        owner,
        group,
        linux::AT_SYMLINK_NOFOLLOW,
    )
}

/// `faccessat(2)`
pub fn sys_linux_faccessat(
    dirfd: Fd,
    pathname_ptr: UserPtr<u8>,
    _mode: usize,
    _flags: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname_ptr, _mode, _flags) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname_ptr);
        match crate::modules::posix::fs::faccessat(fs_id, &dir_path, &path) {
            Ok(_) => 0,
            Err(err) => linux_errno(err.code()),
        }
    })
}

pub fn sys_linux_faccessat2(
    dirfd: Fd,
    pathname_ptr: UserPtr<u8>,
    mode: usize,
    flags: usize,
) -> usize {
    sys_linux_faccessat(dirfd, pathname_ptr, mode, flags)
}

pub fn sys_linux_fchmod(fd: Fd, mode: usize) -> usize {
    crate::require_posix_fs!((fd, mode) => {
        match crate::modules::posix::fs::fchmod(fd.as_u32(), mode as u16) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fchown(fd: Fd, owner: usize, group: usize) -> usize {
    crate::require_posix_fs!((fd, owner, group) => {
        match crate::modules::posix::fs::fchown(fd.as_u32(), owner as u32, group as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fchmodat(dirfd: Fd, pathname: UserPtr<u8>, mode: usize, _flags: usize) -> usize {
    crate::require_posix_fs!((dirfd, pathname, mode, _flags) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::chmod(fs_id, &resolved, mode as u16) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fchownat(
    dirfd: Fd,
    pathname: UserPtr<u8>,
    owner: usize,
    group: usize,
    _flags: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname, owner, group, _flags) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::chown(fs_id, &resolved, owner as u32, group as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `fstat(2)`
pub fn sys_linux_fstat(fd: Fd, statbuf: UserPtr<LinuxStat>) -> usize {
    crate::require_posix_fs!((fd, statbuf) => {
        if fd.as_usize() == linux::FB_FD {
            let mut lstat: LinuxStat = unsafe { core::mem::zeroed() };
            lstat.st_mode = linux::S_IFCHR | 0o666;
            lstat.st_blksize = LinuxCompatConfig::STAT_BLKSIZE as i64;
            return write_user_struct!(statbuf, lstat);
        }

        match crate::modules::posix::fs::fstat(fd.as_u32()) {
            Ok(pstat) => write_user_struct!(statbuf, fill_linux_stat(pstat)),
            Err(err) => linux_errno(err.code()),
        }
    })
}

/// `newfstatat(2)` / `fstatat(2)`
pub fn sys_linux_newfstatat(
    dirfd: Fd,
    pathname: UserPtr<u8>,
    statbuf: UserPtr<LinuxStat>,
    flags: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname, statbuf, flags) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let follow = (flags & linux::AT_SYMLINK_NOFOLLOW) == 0;
        match crate::modules::posix::fs::fstatat(fs_id, &dir_path, &path, follow) {
            Ok(pstat) => write_user_struct!(statbuf, fill_linux_stat(pstat)),
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `statx(2)` — Extended stat (Modern).
pub fn sys_linux_statx(
    dirfd: Fd,
    pathname: UserPtr<u8>,
    flags: usize,
    mask: usize,
    statxbuf: UserPtr<LinuxStatx>,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname, flags, mask, statxbuf) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let follow = (flags & linux::AT_SYMLINK_NOFOLLOW) == 0;
        match crate::modules::posix::fs::fstatat(fs_id, &dir_path, &path, follow) {
            Ok(pstat) => {
                let mut stx: LinuxStatx = unsafe { core::mem::zeroed() };
                stx.stx_mask = linux::STATX_BASIC_STATS & mask as u32;
                stx.stx_blksize = LinuxCompatConfig::STAT_BLKSIZE;
                stx.stx_nlink = 1;
                stx.stx_uid = pstat.uid;
                stx.stx_gid = pstat.gid;
                stx.stx_mode = pstat.mode;

                if pstat.is_dir { stx.stx_mode |= linux::S_IFDIR as u16; }
                else if pstat.is_symlink { stx.stx_mode |= linux::S_IFLNK as u16; }
                else { stx.stx_mode |= linux::S_IFREG as u16; }

                stx.stx_ino = pstat.ino;
                stx.stx_size = pstat.size;
                stx.stx_blocks = (pstat.size + (LinuxCompatConfig::STAT_BLOCK_SIZE - 1)) / LinuxCompatConfig::STAT_BLOCK_SIZE;
                stx.stx_atime = LinuxStatxTimestamp { tv_sec: pstat.atime, tv_nsec: 0, __reserved: 0 };
                stx.stx_mtime = LinuxStatxTimestamp { tv_sec: pstat.mtime, tv_nsec: 0, __reserved: 0 };
                stx.stx_ctime = LinuxStatxTimestamp { tv_sec: pstat.ctime, tv_nsec: 0, __reserved: 0 };
                stx.stx_btime = stx.stx_ctime;

                write_user_struct!(statxbuf, stx)
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `statfs(2)` / `fstatfs(2)`
pub fn sys_linux_statfs(path_ptr: UserPtr<u8>, buf: UserPtr<LinuxStatfs>) -> usize {
    crate::require_posix_fs!((path_ptr, buf) => {
        let (fs_id, _, _) = resolve_at!(Fd(linux::AT_FDCWD as i32), path_ptr);
        match crate::modules::posix::fs::statfs(fs_id) {
            Ok(s) => write_user_struct!(buf, fill_linux_statfs(fs_id, s)),
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fstatfs(fd: Fd, buf: UserPtr<LinuxStatfs>) -> usize {
    crate::require_posix_fs!((fd, buf) => {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd.as_u32()) {
            Ok(id) => id,
            Err(_) => return linux_errno(crate::modules::posix_consts::errno::EBADF),
        };
        match crate::modules::posix::fs::statfs(fs_id) {
            Ok(s) => write_user_struct!(buf, fill_linux_statfs(fs_id, s)),
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `utimes(2)` / `utimensat(2)`
pub fn sys_linux_utimensat(
    dirfd: Fd,
    pathname: UserPtr<u8>,
    times: UserPtr<LinuxTimespec>,
    _flags: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname, times, _flags) => {
        syscall_path_at!(dirfd, pathname, write, fs_id, dir_path, path, resolved);

        if times.is_null() {
            match crate::modules::posix::fs::utimensat(fs_id, &resolved) {
                Ok(()) => 0,
                Err(e) => linux_errno(e.code()),
            }
        } else {
            let ts_atime = match times.read() { Ok(v) => v, Err(e) => return e };
            let ts_mtime = match times.add(1).read() { Ok(v) => v, Err(e) => return e };
            let atime = crate::modules::posix::time::PosixTimespec { sec: ts_atime.tv_sec, nsec: ts_atime.tv_nsec as i32 };
            let mtime = crate::modules::posix::time::PosixTimespec { sec: ts_mtime.tv_sec, nsec: ts_mtime.tv_nsec as i32 };
            match crate::modules::posix::fs::utimes(fs_id, &resolved, atime, mtime) {
                Ok(()) => 0,
                Err(e) => linux_errno(e.code()),
            }
        }
    })
}

pub fn sys_linux_utime(filename: UserPtr<u8>, _times: UserPtr<u8>) -> usize {
    sys_linux_utimensat(Fd(linux::AT_FDCWD as i32), filename, UserPtr::new(0), 0)
}

/// ── Symbolic Link Operations ────────────────────────────────────────────────

pub fn sys_linux_readlink(pathname: UserPtr<u8>, buf: UserPtr<u8>, bufsiz: usize) -> usize {
    sys_linux_readlinkat(Fd(linux::AT_FDCWD as i32), pathname, buf, bufsiz)
}

pub fn sys_linux_stat(pathname: UserPtr<u8>, statbuf: UserPtr<LinuxStat>) -> usize {
    sys_linux_newfstatat(Fd(linux::AT_FDCWD as i32), pathname, statbuf, 0)
}

pub fn sys_linux_lstat(pathname: UserPtr<u8>, statbuf: UserPtr<LinuxStat>) -> usize {
    sys_linux_newfstatat(
        Fd(linux::AT_FDCWD as i32),
        pathname,
        statbuf,
        linux::AT_SYMLINK_NOFOLLOW,
    )
}

pub fn sys_linux_mkdir(pathname: UserPtr<u8>, mode: usize) -> usize {
    sys_linux_mkdirat(Fd(linux::AT_FDCWD as i32), pathname, mode)
}

pub fn sys_linux_truncate(pathname: UserPtr<u8>, len: usize) -> usize {
    crate::require_posix_fs!((pathname, len) => {
        let (fs_id, dir_path, path) = resolve_at!(Fd(linux::AT_FDCWD as i32), pathname);
        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::truncate(fs_id, &resolved, len) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_umask(mask: usize) -> usize {
    crate::require_posix_fs!((mask) => {
        crate::modules::posix::fs::umask((mask & 0o777) as u16) as usize
    })
}

pub fn sys_linux_readlinkat(
    dirfd: Fd,
    pathname: UserPtr<u8>,
    buf: UserPtr<u8>,
    bufsiz: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname, buf, bufsiz) => {
        let (fs_id, _dir_path, path) = resolve_at!(dirfd, pathname);
        match crate::modules::posix::fs::readlink(fs_id, &path) {
            Ok(target) => {
                let bytes = target.as_bytes();
                let len = bytes.len().min(bufsiz);
                buf.write_bytes(&bytes[..len]).map(|_| len).unwrap_or_else(|e| e)
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// ── Internal Helpers ────────────────────────────────────────────────────────

fn fill_linux_stat(pstat: crate::modules::posix::fs::PosixStat) -> LinuxStat {
    let mut lstat: LinuxStat = unsafe { core::mem::zeroed() };
    lstat.st_ino = pstat.ino;
    lstat.st_mode = pstat.mode as u32;
    if pstat.is_dir {
        lstat.st_mode |= linux::S_IFDIR;
    } else if pstat.is_symlink {
        lstat.st_mode |= linux::S_IFLNK;
    } else {
        lstat.st_mode |= linux::S_IFREG;
    }

    lstat.st_nlink = 1;
    lstat.st_uid = pstat.uid;
    lstat.st_gid = pstat.gid;
    lstat.st_size = pstat.size as i64;
    lstat.st_blksize = LinuxCompatConfig::STAT_BLKSIZE as i64;
    lstat.st_blocks = (pstat.size as i64 + (LinuxCompatConfig::STAT_BLOCK_SIZE as i64 - 1))
        / LinuxCompatConfig::STAT_BLOCK_SIZE as i64;
    lstat.st_atime = pstat.atime;
    lstat.st_mtime = pstat.mtime;
    lstat.st_ctime = pstat.ctime;
    lstat
}

fn fill_linux_statfs(fs_id: u32, s: crate::modules::posix::fs::PosixFsStats) -> LinuxStatfs {
    let free_ram_pages = crate::modules::allocators::bitmap_pmm::get_free_pages() as u64;
    let total_ram_pages = crate::modules::allocators::bitmap_pmm::PMM_TOTAL_PAGES as u64;

    LinuxStatfs {
        f_type: (if fs_id == 1 {
            linux::RAMFS_MAGIC
        } else {
            0xbeef_fade
        }) as i64,
        f_bsize: 4096,
        f_blocks: total_ram_pages,
        f_bfree: free_ram_pages,
        f_bavail: free_ram_pages,
        f_files: s.f_files,
        f_ffree: s.f_ffree,
        f_fsid: [fs_id as i32, 0],
        f_namelen: s.f_namelen as i64,
        f_frsize: 4096,
        f_flags: 0,
        f_spare: [0; 4],
    }
}
