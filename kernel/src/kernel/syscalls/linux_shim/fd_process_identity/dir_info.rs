use super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use alloc::collections::BTreeMap;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
use spin::Mutex;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
static GETDENTS64_CURSOR: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());

#[cfg(not(feature = "linux_compat"))]
const UTSNAME_FIELD_LEN: usize = 65;

#[repr(C, packed)]
#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy, Default)]
struct LinuxDirent64Header {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
}

#[cfg(not(feature = "linux_compat"))]
const DIRENT64_FIXED: usize = core::mem::size_of::<LinuxDirent64Header>();

#[cfg(all(not(feature = "linux_compat"), feature = "posix_fs"))]
pub(crate) fn clear_getdents_cursor(fd: u32) {
    GETDENTS64_CURSOR.lock().remove(&fd);
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getdents64(fd: usize, dirp: usize, count: usize) -> usize {
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
                let hdr = LinuxDirent64Header {
                    d_ino: 1,
                    d_off: (idx + 1) as i64,
                    d_reclen: reclen as u16,
                    d_type: 0,
                };
                let hdr_bytes = unsafe {
                    core::slice::from_raw_parts(
                        (&hdr as *const LinuxDirent64Header).cast::<u8>(),
                        DIRENT64_FIXED,
                    )
                };

                dst[off..off + DIRENT64_FIXED].copy_from_slice(hdr_bytes);
                let name_start = off + DIRENT64_FIXED;
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
    sysname: [u8; UTSNAME_FIELD_LEN],
    nodename: [u8; UTSNAME_FIELD_LEN],
    release: [u8; UTSNAME_FIELD_LEN],
    version: [u8; UTSNAME_FIELD_LEN],
    machine: [u8; UTSNAME_FIELD_LEN],
    domainname: [u8; UTSNAME_FIELD_LEN],
}

#[cfg(not(feature = "linux_compat"))]
const UTSNAME_SYSNAME_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, sysname);
#[cfg(not(feature = "linux_compat"))]
const UTSNAME_NODENAME_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, nodename);
#[cfg(not(feature = "linux_compat"))]
const UTSNAME_RELEASE_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, release);
#[cfg(not(feature = "linux_compat"))]
const UTSNAME_VERSION_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, version);
#[cfg(not(feature = "linux_compat"))]
const UTSNAME_MACHINE_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, machine);
#[cfg(not(feature = "linux_compat"))]
const UTSNAME_DOMAINNAME_OFFSET: usize = core::mem::offset_of!(LinuxUtsname, domainname);

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_uname(buf: usize) -> usize {
    let total = core::mem::size_of::<LinuxUtsname>();
    with_user_write_bytes(buf, total, |dst| {
        dst.fill(0);
        let copy_field = |dst: &mut [u8], offset: usize, value: &[u8]| {
            let len = core::cmp::min(value.len(), UTSNAME_FIELD_LEN - 1);
            dst[offset..offset + len].copy_from_slice(&value[..len]);
        };
        copy_field(dst, UTSNAME_SYSNAME_OFFSET, b"AetherCore");
        copy_field(dst, UTSNAME_NODENAME_OFFSET, b"aethercore");
        copy_field(dst, UTSNAME_RELEASE_OFFSET, b"0.2.0");
        copy_field(dst, UTSNAME_VERSION_OFFSET, b"#1 SMP");
        #[cfg(target_arch = "x86_64")]
        copy_field(dst, UTSNAME_MACHINE_OFFSET, b"x86_64");
        #[cfg(target_arch = "aarch64")]
        copy_field(dst, UTSNAME_MACHINE_OFFSET, b"aarch64");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        copy_field(dst, UTSNAME_MACHINE_OFFSET, b"unknown");
        copy_field(dst, UTSNAME_DOMAINNAME_OFFSET, b"(none)");
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}
