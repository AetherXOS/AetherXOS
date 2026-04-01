use super::super::*;
use crate::kernel::syscalls::with_user_write_bytes;

const RENAME_NOREPLACE: usize = 1;
const RENAME_EXCHANGE: usize = 2;

/// `getdents64(2)` — Read directory entries.
pub fn sys_linux_getdents64(fd: Fd, dirp: UserPtr<u8>, count: usize) -> usize {
    crate::require_posix_fs!((fd, dirp, count) => {
        let fd_u32 = fd.as_u32();
        let path = match crate::modules::posix::fs::fd_path(fd_u32) { Ok(p) => p, Err(e) => return linux_errno(e.code()) };
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd_u32) { Ok(id) => id, Err(e) => return linux_errno(e.code()) };
        let fd_offset = match crate::modules::posix::fs::lseek(fd_u32, 0, crate::modules::posix::fs::SeekWhence::Cur) { Ok(o) => o, Err(e) => return linux_errno(e.code()) };

        let entries = match crate::modules::posix::fs::scandir(fs_id, &path) { Ok(e) => e, Err(err) => return linux_errno(err.code()) };

        let mut full_entries = alloc::vec::Vec::new();
        if fd_offset == 0 {
            full_entries.push(alloc::string::String::from("."));
            full_entries.push(alloc::string::String::from(".."));
        }
        for e in entries { full_entries.push(e); }

        let mut out_offset: usize = 0;
        let mut items_processed = 0;
        let mut current_idx = fd_offset as usize;

        while current_idx < full_entries.len() {
            let name = &full_entries[current_idx];
            let name_bytes = name.as_bytes();
            let base_len = linux::DIRENT64_BASE_SIZE;
            let reclen = (base_len + name_bytes.len() + 1 + 7) & !7;

            if out_offset + reclen > count {
                if items_processed == 0 { return linux_inval(); }
                break;
            }

            let d_type: u8 = if name == "." || name == ".." { linux::DT_DIR } else {
                let cp = if path == "/" { alloc::format!("/{}", name) } else { alloc::format!("{}/{}", path, name) };
                match crate::modules::posix::fs::stat(fs_id, &cp) {
                    Ok(st) => if st.is_dir { linux::DT_DIR } else if st.is_symlink { linux::DT_LNK } else { linux::DT_REG },
                    Err(_) => linux::DT_UNKNOWN,
                }
            };

            let dirent = LinuxDirent64 {
                d_ino: (current_idx as u64) + 1,
                d_off: (current_idx as i64) + 1,
                d_reclen: reclen as u16,
                d_type,
            };

            let _ = with_user_write_bytes(dirp.addr + out_offset, base_len, |dst| {
                let ptr = &dirent as *const _ as *const u8;
                dst.copy_from_slice(unsafe { core::slice::from_raw_parts(ptr, base_len) });
                0
            });
            let _ = with_user_write_bytes(dirp.addr + out_offset + base_len, name_bytes.len() + 1, |dst| {
                dst[..name_bytes.len()].copy_from_slice(name_bytes);
                dst[name_bytes.len()] = 0;
                0
            });

            out_offset += reclen;
            items_processed += 1;
            current_idx += 1;
        }

        if items_processed > 0 {
            let _ = crate::modules::posix::fs::lseek(fd_u32, current_idx as i64, crate::modules::posix::fs::SeekWhence::Set);
        }
        out_offset
    })
}

/// `mkdirat(2)`
pub fn sys_linux_mkdirat(dirfd: Fd, pathname: UserPtr<u8>, mode: usize) -> usize {
    crate::require_posix_fs!((dirfd, pathname, mode) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname);
        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) { Ok(p) => p, Err(e) => return linux_errno(e.code()) };
        if super::mount::linux_path_is_readonly(&resolved) { return linux_errno(crate::modules::posix_consts::errno::EROFS); }
        match crate::modules::posix::fs::mkdir(fs_id, &resolved, mode as u16) { Ok(()) => 0, Err(e) => linux_errno(e.code()) }
    })
}

/// `rmdir(2)`
pub fn sys_linux_rmdir(pathname: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((pathname) => {
        let (fs_id, dir_path, path) = resolve_at!(Fd(linux::AT_FDCWD as i32), pathname);
        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) { Ok(p) => p, Err(e) => return linux_errno(e.code()) };
        if super::mount::linux_path_is_readonly(&resolved) { return linux_errno(crate::modules::posix_consts::errno::EROFS); }
        match crate::modules::posix::fs::rmdir(fs_id, &resolved) { Ok(()) => 0, Err(e) => linux_errno(e.code()) }
    })
}

/// `renameat2(2)` — Rename with flags (e.g. RENAME_NOREPLACE, RENAME_EXCHANGE).
pub fn sys_linux_renameat2(
    olddirfd: Fd,
    oldpath: UserPtr<u8>,
    newdirfd: Fd,
    newpath: UserPtr<u8>,
    flags: usize,
) -> usize {
    crate::require_posix_fs!((olddirfd, oldpath, newdirfd, newpath, flags) => {
        if flags != 0 && (flags & !(RENAME_NOREPLACE | RENAME_EXCHANGE)) != 0 { return linux_inval(); }

        let (old_fs, old_dir, old_rel) = resolve_at!(olddirfd, oldpath);
        let (new_fs, new_dir, new_rel) = resolve_at!(newdirfd, newpath);

        let old_abs = match crate::modules::posix::fs::resolve_at_path(old_fs, &old_dir, &old_rel) { Ok(p) => p, Err(e) => return linux_errno(e.code()) };
        let new_abs = match crate::modules::posix::fs::resolve_at_path(new_fs, &new_dir, &new_rel) { Ok(p) => p, Err(e) => return linux_errno(e.code()) };

        if super::mount::linux_path_is_readonly(&old_abs) || super::mount::linux_path_is_readonly(&new_abs) {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }

        match crate::modules::posix::fs::rename(old_fs, &old_abs, &new_abs) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_chdir(pathname: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((pathname) => {
        let (fs_id, _, path) = resolve_at!(Fd(linux::AT_FDCWD as i32), pathname);
        match crate::modules::posix::fs::chdir(fs_id, &path) {
            Ok(()) => {
                #[cfg(feature = "posix_process")]
                crate::modules::posix::process::setenv("PWD", &path, true).ok();
                0
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fchdir(fd: Fd) -> usize {
    crate::require_posix_fs!((fd) => {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd.as_u32()) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        let path = match crate::modules::posix::fs::fd_path(fd.as_u32()) {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };

        match crate::modules::posix::fs::chdir(fs_id, &path) {
            Ok(()) => {
                #[cfg(feature = "posix_process")]
                crate::modules::posix::process::setenv("PWD", &path, true).ok();
                0
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_getcwd(buf: UserPtr<u8>, size: usize) -> usize {
    crate::require_posix_process!((buf, size) => {
        let pwd = crate::modules::posix::process::getenv("PWD").unwrap_or_else(|| alloc::string::String::from("/"));
        let bytes = pwd.as_bytes();
        if size < bytes.len() + 1 { return linux_errno(crate::modules::posix_consts::errno::ERANGE); }
        with_user_write_bytes(buf.addr, bytes.len() + 1, |dst| {
            dst[..bytes.len()].copy_from_slice(bytes);
            dst[bytes.len()] = 0;
            0
        }).map(|_| buf.addr).unwrap_or_else(|_| linux_fault())
    })
}
