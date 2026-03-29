use super::super::*;
use alloc::format;

pub fn sys_linux_link(oldpath: UserPtr<u8>, newpath: UserPtr<u8>) -> usize {
    sys_linux_linkat(
        Fd(linux::AT_FDCWD as i32),
        oldpath,
        Fd(linux::AT_FDCWD as i32),
        newpath,
        0,
    )
}

pub fn sys_linux_linkat(
    olddirfd: Fd,
    oldpath: UserPtr<u8>,
    newdirfd: Fd,
    newpath: UserPtr<u8>,
    _flags: usize,
) -> usize {
    crate::require_posix_fs!((olddirfd, oldpath, newdirfd, newpath, _flags) => {
        let (fs_id, dir_old, old) = resolve_at!(olddirfd, oldpath);
                let (_, dir_new, new) = resolve_at!(newdirfd, newpath);

                let abs_old = if old.starts_with('/') { old } else { format!("{}/{}", dir_old, old) };
                let abs_new = if new.starts_with('/') { new } else { format!("{}/{}", dir_new, new) };
                if super::mount::linux_path_is_readonly(&abs_old) || super::mount::linux_path_is_readonly(&abs_new) {
                    return linux_errno(crate::modules::posix_consts::errno::EROFS);
                }

                match crate::modules::posix::fs::link(fs_id, &abs_old, &abs_new) {
                    Ok(()) => 0,
                    Err(e) => linux_errno(e.code())
                }
    })
}

pub fn sys_linux_symlink(target: UserPtr<u8>, linkpath: UserPtr<u8>) -> usize {
    sys_linux_symlinkat(target, Fd(linux::AT_FDCWD as i32), linkpath)
}

pub fn sys_linux_symlinkat(target: UserPtr<u8>, newdirfd: Fd, linkpath: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((target, newdirfd, linkpath) => {
        let tgt = match read_user_c_string(target.addr, crate::config::KernelConfig::vfs_max_mount_path()) { Ok(p) => p, Err(e) => return e };
                let (fs_id, dir_new, new) = resolve_at!(newdirfd, linkpath);

                let abs_new = if new.starts_with('/') { new } else { format!("{}/{}", dir_new, new) };
                if super::mount::linux_path_is_readonly(&abs_new) {
                    return linux_errno(crate::modules::posix_consts::errno::EROFS);
                }

                match crate::modules::posix::fs::symlink(fs_id, &tgt, &abs_new) {
                    Ok(()) => 0,
                    Err(e) => linux_errno(e.code())
                }
    })
}

pub fn sys_linux_unlink(pathname: UserPtr<u8>) -> usize {
    sys_linux_unlinkat(Fd(linux::AT_FDCWD as i32), pathname, 0)
}

pub fn sys_linux_mknod(pathname: UserPtr<u8>, mode: usize, dev: usize) -> usize {
    sys_linux_mknodat(Fd(linux::AT_FDCWD as i32), pathname, mode, dev)
}

pub fn sys_linux_mknodat(dirfd: Fd, pathname: UserPtr<u8>, mode: usize, _dev: usize) -> usize {
    crate::require_posix_fs!((dirfd, pathname, mode, _dev) => {
        let file_type = (mode as u32) & linux::S_IFMT;
        if file_type != 0 && file_type != linux::S_IFREG {
            return linux_inval();
        }

        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let abs_path = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        if super::mount::linux_path_is_readonly(&abs_path) {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }
        match crate::modules::posix::fs::access(fs_id, &abs_path) {
            Ok(true) => return linux_errno(crate::modules::posix_consts::errno::EEXIST),
            Ok(false) => {}
            Err(e) => return linux_errno(e.code()),
        }

        let fd = match crate::modules::posix::fs::openat(fs_id, &dir_path, &path, true) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        let _ = crate::modules::posix::fs::fchmod(fd, (mode & 0o7777) as u16);
        let _ = crate::modules::posix::fs::close(fd);
        0
    })
}

pub fn sys_linux_unlinkat(dirfd: Fd, pathname: UserPtr<u8>, flags: usize) -> usize {
    crate::require_posix_fs!((dirfd, pathname, flags) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
                let abs_path = if path.starts_with('/') { path } else { format!("{}/{}", dir_path, path) };
                if super::mount::linux_path_is_readonly(&abs_path) {
                    return linux_errno(crate::modules::posix_consts::errno::EROFS);
                }

                if (flags & linux::AT_REMOVEDIR) != 0 {
                    match crate::modules::posix::fs::rmdir(fs_id, &abs_path) { Ok(()) => 0, Err(e) => linux_errno(e.code()) }
                } else {
                    match crate::modules::posix::fs::unlink(fs_id, &abs_path) { Ok(()) => 0, Err(e) => linux_errno(e.code()) }
                }
    })
}

pub fn sys_linux_rename(oldpath: UserPtr<u8>, newpath: UserPtr<u8>) -> usize {
    sys_linux_renameat(
        Fd(linux::AT_FDCWD as i32),
        oldpath,
        Fd(linux::AT_FDCWD as i32),
        newpath,
    )
}

pub fn sys_linux_renameat(
    olddirfd: Fd,
    oldpath: UserPtr<u8>,
    newdirfd: Fd,
    newpath: UserPtr<u8>,
) -> usize {
    crate::require_posix_fs!((olddirfd, oldpath, newdirfd, newpath) => {
        let (fs_id, dir_old, old) = resolve_at!(olddirfd, oldpath);
                let (_, dir_new, new) = resolve_at!(newdirfd, newpath);

                let abs_old = if old.starts_with('/') { old } else { format!("{}/{}", dir_old, old) };
                let abs_new = if new.starts_with('/') { new } else { format!("{}/{}", dir_new, new) };
                if super::mount::linux_path_is_readonly(&abs_old) || super::mount::linux_path_is_readonly(&abs_new) {
                    return linux_errno(crate::modules::posix_consts::errno::EROFS);
                }

                match crate::modules::posix::fs::rename(fs_id, &abs_old, &abs_new) {
                    Ok(()) => 0,
                    Err(e) => linux_errno(e.code())
                }
    })
}
