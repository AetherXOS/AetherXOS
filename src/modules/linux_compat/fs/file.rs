use super::super::*;
use core::fmt::Write;
use core::sync::atomic::{AtomicU64, Ordering};

#[path = "fd_ops.rs"]
mod fd_ops;
#[path = "file_flags.rs"]
mod file_flags;
use file_flags::{
    apply_linux_open_post_flags, build_linux_tmpfile_path,
    linux_tmpfile_write_mode_valid,
};
pub(crate) use file_flags::decode_linux_open_intent;
pub use fd_ops::{
    sys_linux_close, sys_linux_close_range, sys_linux_dup, sys_linux_dup2, sys_linux_dup3,
    sys_linux_pipe, sys_linux_pipe2,
};

const STDIO_WRITE_CHUNK_LIMIT: usize = 4096;
const SENDFILE_CHUNK_LIMIT: usize = 64 * 1024;
static NEXT_LINUX_TMPFILE_ID: AtomicU64 = AtomicU64::new(1);

pub fn sys_linux_fdatasync(fd: Fd) -> usize {
    crate::require_posix_fs!((fd) => {
        match crate::modules::posix::fs::fdatasync(fd.as_u32()) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_fallocate(fd: Fd, mode: usize, offset: usize, len: usize) -> usize {
    crate::require_posix_fs!((fd, mode, offset, len) => {
        if super::mount::linux_fd_is_readonly(fd) {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }
        match crate::modules::posix::fs::fallocate(fd.as_u32(), mode as u32, offset, len) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_sync() -> usize {
    crate::require_posix_fs!(() => {
        let fs_ids: alloc::vec::Vec<u32> = {
            let table = crate::modules::posix::fs::FS_CONTEXTS.lock();
            table.keys().copied().collect()
        };
        for fs_id in fs_ids {
            let _ = crate::modules::posix::fs::syncfs(fs_id);
        }
        0
    })
}

pub fn sys_linux_syncfs(fd: Fd) -> usize {
    crate::require_posix_fs!((fd) => {
        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd.as_u32()) {
            Ok(id) => id,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::syncfs(fs_id) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}


/// `openat(2)` — Open a file relative to a directory file descriptor.
pub fn sys_linux_openat(dirfd: Fd, pathname_ptr: UserPtr<u8>, flags: usize, _mode: usize) -> usize {
    crate::require_posix_fs!((dirfd, pathname_ptr, flags, _mode) => {
        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname_ptr);

        // Handle special device simulation
        if path == "/dev/fb0" { return linux::FB_FD; }
        if path == "/dev/input/event0" { return linux::INPUT_FD; }

        let intent = match decode_linux_open_intent(flags) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let resolved = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
            Ok(p) => p,
            Err(e) => return linux_errno(e.code())
        };

        if (intent.create || intent.trunc || intent.tmpfile)
            && super::mount::linux_path_is_readonly(&resolved)
        {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }

        if intent.tmpfile {
            let id = NEXT_LINUX_TMPFILE_ID.fetch_add(1, Ordering::Relaxed);
            let anon_path = build_linux_tmpfile_path(&resolved, id);
            return match crate::modules::posix::fs::openat(fs_id, "/", &anon_path, true) {
                Ok(fd) => {
                    apply_linux_open_post_flags(fd, flags);
                    fd as usize
                }
                Err(err) => linux_errno(err.code()),
            };
        }

        match crate::modules::posix::fs::openat(fs_id, &dir_path, &path, intent.create) {
            Ok(fd) => {
                if intent.trunc {
                    let _ = crate::modules::posix::fs::ftruncate(fd, 0);
                }
                apply_linux_open_post_flags(fd, flags);
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmpfile_requires_write_access_mode() {
        let ro_flags = linux::open_flags::O_TMPFILE;
        let wo_flags = linux::open_flags::O_TMPFILE | linux::open_flags::O_WRONLY;
        let rw_flags = linux::open_flags::O_TMPFILE | linux::open_flags::O_RDWR;

        assert!(!linux_tmpfile_write_mode_valid(ro_flags));
        assert!(linux_tmpfile_write_mode_valid(wo_flags));
        assert!(linux_tmpfile_write_mode_valid(rw_flags));
    }

    #[test]
    fn build_tmpfile_path_is_root_safe() {
        assert_eq!(build_linux_tmpfile_path("/", 7), "/.tmpfile-7");
        assert_eq!(build_linux_tmpfile_path("/tmp", 11), "/tmp/.tmpfile-11");
        assert_eq!(build_linux_tmpfile_path("/tmp/", 13), "/tmp/.tmpfile-13");
    }
}

pub fn sys_linux_read(fd: Fd, ptr: UserPtr<u8>, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if fd.as_usize() == linux::STDOUT_FILENO || fd.as_usize() == linux::STDERR_FILENO {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }

    #[cfg(feature = "posix_pipe")]
    if fd.as_usize() >= linux::PIPE_BASE_FD {
        let mut n = 0;
        return match ptr.write_bytes_with(len, |dst| {
            match crate::modules::posix::pipe::read(fd.as_u32(), dst) {
                Ok(read) => {
                    n = read;
                    0
                }
                Err(e) => linux_errno(e as i32),
            }
        }) {
            Ok(0) => n,
            Ok(e) => e,
            Err(e) => e,
        };
    }

    crate::require_posix_fs!((fd, ptr, len) => {
        match ptr.write_bytes_with(len, |dst| {
            match crate::modules::posix::fs::read(fd.as_u32(), dst) {
                Ok(n) => n,
                Err(err) => linux_errno(err.code())
            }
        }) {
            Ok(n) => n,
            Err(e) => e,
        }
    })
}

pub fn sys_linux_write(fd: Fd, ptr: UserPtr<u8>, len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    // STDIO handling
    if fd.as_usize() == linux::STDOUT_FILENO || fd.as_usize() == linux::STDERR_FILENO {
        let mut n = 0;
        return match ptr.read_bytes_with_limit(len, STDIO_WRITE_CHUNK_LIMIT, |slice| {
            if let Ok(s) = core::str::from_utf8(slice) {
                let _ = crate::hal::serial::SERIAL1.lock().write_str(s);
                n = slice.len();
                0
            } else {
                linux_inval()
            }
        }) {
            Ok(0) => n,
            Ok(e) => e,
            Err(e) => e,
        };
    }

    if super::mount::linux_fd_is_readonly(fd) {
        return linux_errno(crate::modules::posix_consts::errno::EROFS);
    }

    #[cfg(feature = "posix_pipe")]
    if fd.as_usize() >= linux::PIPE_BASE_FD {
        let mut n = 0;
        return match ptr.read_bytes_with(len, |src| {
            match crate::modules::posix::pipe::write(fd.as_u32(), src) {
                Ok(wrote) => {
                    n = wrote;
                    0
                }
                Err(e) => linux_errno(e as i32),
            }
        }) {
            Ok(0) => n,
            Ok(e) => e,
            Err(e) => e,
        };
    }

    crate::require_posix_fs!((fd, ptr, len) => {
        match ptr.read_bytes_with(len, |src| {
            match crate::modules::posix::fs::write(fd.as_u32(), src) {
                Ok(n) => n,
                Err(err) => linux_errno(err.code())
            }
        }) {
            Ok(n) => n,
            Err(e) => e,
        }
    })
}

pub fn sys_linux_open(pathname: UserPtr<u8>, flags: usize, mode: usize) -> usize {
    sys_linux_openat(Fd(linux::AT_FDCWD as i32), pathname, flags, mode)
}

#[path = "file_io_ext.rs"]
mod file_io_ext;

pub use file_io_ext::{
    sys_linux_copy_file_range, sys_linux_fsync, sys_linux_ftruncate, sys_linux_lseek,
    sys_linux_pread64, sys_linux_pwrite64, sys_linux_readv, sys_linux_sendfile, sys_linux_splice,
    sys_linux_tee, sys_linux_vmsplice, sys_linux_writev,
};
