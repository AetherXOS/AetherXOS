use super::super::*;
use crate::modules::linux_compat::{linux, linux_inval};

#[derive(Clone, Copy)]
pub(crate) struct LinuxOpenIntent {
    pub create: bool,
    pub trunc: bool,
    pub tmpfile: bool,
}

#[inline(always)]
pub(super) fn linux_open_access_mode(flags: usize) -> usize {
    flags & linux::open_flags::O_ACCMODE
}

#[inline(always)]
pub(super) fn linux_tmpfile_requested(flags: usize) -> bool {
    (flags & linux::open_flags::O_TMPFILE) == linux::open_flags::O_TMPFILE
}

#[inline(always)]
pub(super) fn linux_tmpfile_write_mode_valid(flags: usize) -> bool {
    matches!(
        linux_open_access_mode(flags),
        linux::open_flags::O_WRONLY | linux::open_flags::O_RDWR
    )
}

pub(super) fn build_linux_tmpfile_path(dir: &str, id: u64) -> alloc::string::String {
    if dir == "/" {
        alloc::format!("/.tmpfile-{id}")
    } else {
        let trimmed = dir.trim_end_matches('/');
        alloc::format!("{trimmed}/.tmpfile-{id}")
    }
}

pub(super) fn apply_linux_open_post_flags(fd: u32, flags: usize) {
    if (flags & linux::open_flags::O_APPEND) != 0 {
        let current = crate::modules::posix::fs::fcntl_get_status_flags(fd).unwrap_or(0);
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(
            fd,
            current | crate::modules::posix_consts::fs::O_APPEND as u32,
        );
        let _ = crate::modules::posix::fs::lseek(fd, 0, crate::modules::posix::fs::SeekWhence::End);
    }
    if (flags & linux::open_flags::O_NONBLOCK) != 0 {
        let current = crate::modules::posix::fs::fcntl_get_status_flags(fd).unwrap_or(0);
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(
            fd,
            current | crate::modules::posix_consts::net::O_NONBLOCK as u32,
        );
    }
    if (flags & linux::open_flags::O_CLOEXEC) != 0 {
        super::io::linux_fd_set_descriptor_flags(fd, super::io::LINUX_FD_CLOEXEC);
    } else {
        super::io::linux_fd_clear_descriptor_flags(fd);
    }
}

pub(crate) fn decode_linux_open_intent(flags: usize) -> Result<LinuxOpenIntent, usize> {
    let create = (flags & linux::open_flags::O_CREAT) != 0;
    let trunc = (flags & linux::open_flags::O_TRUNC) != 0;
    let tmpfile = linux_tmpfile_requested(flags);
    if tmpfile {
        if create || trunc {
            return Err(linux_inval());
        }
        if !linux_tmpfile_write_mode_valid(flags) {
            return Err(linux_inval());
        }
    }
    Ok(LinuxOpenIntent {
        create,
        trunc,
        tmpfile,
    })
}