use super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use alloc::collections::BTreeMap;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use spin::Mutex;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
static GETDENTS64_CURSOR: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(crate) fn clear_getdents_cursor(fd: u32) {
    GETDENTS64_CURSOR.lock().remove(&fd);
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getdents64(fd: usize, dirp: usize, count: usize) -> usize {
    const DIRENT64_FIXED: usize = 19;
    if count < DIRENT64_FIXED {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fd_u32 = match u32::try_from(fd) {
            Ok(v) => v,
            Err(_) => return linux_errno(crate::modules::posix_consts::errno::EBADF),
        };

        let fs_id = match crate::modules::posix::fs::fd_fs_context(fd_u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let path = match crate::modules::posix::fs::fd_path(fd_u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let entries = match crate::modules::posix::fs::scandir(fs_id, &path) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        let mut cursor_tbl = GETDENTS64_CURSOR.lock();
        let cursor = cursor_tbl.entry(fd_u32).or_insert(0usize);
        if *cursor >= entries.len() {
            cursor_tbl.remove(&fd_u32);
            return 0;
        }

        let start_idx = *cursor;
        let write_res = with_user_write_bytes(dirp, count, |dst| {
            let mut idx = start_idx;
            let mut written = 0usize;

            while idx < entries.len() {
                let name = entries[idx].as_bytes();
                let base_len = DIRENT64_FIXED + name.len() + 1;
                let reclen = (base_len + 7) & !7;
                if written + reclen > count {
                    break;
                }

                let off = written;
                let d_ino = 1u64;
                let d_off = (idx + 1) as i64;
                let d_reclen = reclen as u16;
                let d_type = 0u8;

                dst[off..off + 8].copy_from_slice(&d_ino.to_ne_bytes());
                dst[off + 8..off + 16].copy_from_slice(&d_off.to_ne_bytes());
                dst[off + 16..off + 18].copy_from_slice(&d_reclen.to_ne_bytes());
                dst[off + 18] = d_type;
                let name_start = off + 19;
                let name_end = name_start + name.len();
                dst[name_start..name_end].copy_from_slice(name);
                dst[name_end] = 0;
                dst[name_end + 1..off + reclen].fill(0);

                written += reclen;
                idx += 1;
            }

            *cursor = idx;
            written
        });

        match write_res {
            Ok(v) => v,
            Err(_) => linux_errno(crate::modules::posix_consts::errno::EFAULT),
        }
    }

    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, dirp, count);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getcwd(buf: usize, size: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(id) => id,
            Err(err) => return linux_errno(err.code()),
        };
        let cwd = match crate::modules::posix::fs::getcwd(fs_id) {
            Ok(s) => s,
            Err(err) => return linux_errno(err.code()),
        };
        let bytes = cwd.as_bytes();
        if bytes.len() + 1 > size {
            return linux_errno(crate::modules::posix_consts::errno::ERANGE);
        }
        with_user_write_bytes(buf, bytes.len() + 1, |dst| {
            dst[..bytes.len()].copy_from_slice(bytes);
            dst[bytes.len()] = 0;
            buf
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        if size < 2 {
            return linux_errno(crate::modules::posix_consts::errno::ERANGE);
        }
        with_user_write_bytes(buf, 2, |dst| {
            dst[0] = b'/';
            dst[1] = 0;
            buf
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
}

#[repr(C)]
#[cfg(not(feature = "linux_compat"))]
struct LinuxUtsname {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_uname(buf: usize) -> usize {
    let total = core::mem::size_of::<LinuxUtsname>();
    with_user_write_bytes(buf, total, |dst| {
        dst.fill(0);
        let copy_field = |dst: &mut [u8], offset: usize, value: &[u8]| {
            let len = core::cmp::min(value.len(), 64);
            dst[offset..offset + len].copy_from_slice(&value[..len]);
        };
        copy_field(dst, 0, b"HyperCore");
        copy_field(dst, 65, b"hypercore");
        copy_field(dst, 130, b"0.2.0");
        copy_field(dst, 195, b"#1 SMP");
        #[cfg(target_arch = "x86_64")]
        copy_field(dst, 260, b"x86_64");
        #[cfg(target_arch = "aarch64")]
        copy_field(dst, 260, b"aarch64");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        copy_field(dst, 260, b"unknown");
        copy_field(dst, 325, b"(none)");
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}
