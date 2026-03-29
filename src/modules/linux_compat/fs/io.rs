use super::super::*;
use crate::kernel::syscalls::with_user_write_bytes;
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

pub(crate) const LINUX_FD_CLOEXEC: usize = 0x1;
const TIOCGWINSZ: usize = 0x5413;

lazy_static! {
    static ref LINUX_FD_FLAGS: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());
}

pub(crate) fn linux_fd_get_descriptor_flags(fd: u32) -> usize {
    let linux_flags = LINUX_FD_FLAGS.lock().get(&fd).copied().unwrap_or(0);
    let posix_flags =
        crate::modules::posix::fs::fcntl_get_descriptor_flags(fd).unwrap_or(0) as usize;
    (linux_flags | posix_flags) & LINUX_FD_CLOEXEC
}

pub(crate) fn linux_fd_set_descriptor_flags(fd: u32, flags: usize) {
    let masked = flags & LINUX_FD_CLOEXEC;
    let _ = crate::modules::posix::fs::fcntl_set_descriptor_flags(fd, masked as u32);
    let mut table = LINUX_FD_FLAGS.lock();
    if masked == 0 {
        table.remove(&fd);
    } else {
        table.insert(fd, masked);
    }
}

pub(crate) fn linux_fd_clear_descriptor_flags(fd: u32) {
    let _ = crate::modules::posix::fs::fcntl_set_descriptor_flags(fd, 0);
    LINUX_FD_FLAGS.lock().remove(&fd);
}

pub(crate) fn close_cloexec_descriptors() -> usize {
    let fds: alloc::vec::Vec<u32> = {
        let table = crate::modules::posix::fs::FILE_TABLE.lock();
        table
            .iter()
            .filter_map(|(fd, desc)| {
                let linux_flags = LINUX_FD_FLAGS.lock().get(fd).copied().unwrap_or(0);
                if desc.cloexec || (linux_flags & LINUX_FD_CLOEXEC) != 0 {
                    Some(*fd)
                } else {
                    None
                }
            })
            .collect()
    };

    let mut closed = 0usize;
    for fd in fds {
        if super::file::sys_linux_close(Fd(fd as i32)) == 0 {
            linux_fd_clear_descriptor_flags(fd);
            closed = closed.saturating_add(1);
        }
    }
    closed
}

/// `fcntl(2)` — File control operations.
pub fn sys_linux_fcntl(fd: Fd, cmd: usize, arg: usize) -> usize {
    use linux::fcntl as f;

    match cmd {
        f::F_GETFD => {
            return match crate::modules::posix::fs::fcntl_get_descriptor_flags(fd.as_u32()) {
                Ok(flags) => flags as usize & LINUX_FD_CLOEXEC,
                Err(e) => linux_errno(e.code()),
            };
        }
        f::F_SETFD => {
            return match crate::modules::posix::fs::fcntl_set_descriptor_flags(
                fd.as_u32(),
                (arg & LINUX_FD_CLOEXEC) as u32,
            ) {
                Ok(()) => {
                    linux_fd_set_descriptor_flags(fd.as_u32(), arg & LINUX_FD_CLOEXEC);
                    0
                }
                Err(e) => linux_errno(e.code()),
            };
        }
        _ => {}
    }

    crate::require_posix_fs!((fd, cmd, arg) => {
        match cmd {
            f::F_GETFL => {
                let flags = match crate::modules::posix::fs::fcntl_get_status_flags(fd.as_u32()) {
                    Ok(f) => f as usize,
                    Err(_) => {
                        let fd_val = fd.as_usize();
                        if fd_val == linux::STDIN_FILENO { 0 } else { 1 } // Fallback
                    }
                };
                flags
            }
            f::F_SETFL => {
                match crate::modules::posix::fs::fcntl_set_status_flags(fd.as_u32(), arg as u32) {
                    Ok(()) => 0,
                    Err(e) => linux_errno(e.code()),
                }
            }
            f::F_DUPFD | f::F_DUPFD_CLOEXEC => {
                match crate::modules::posix::fs::dup_at_least(fd.as_u32(), arg as u32) {
                    Ok(newfd) => {
                        if cmd == f::F_DUPFD_CLOEXEC {
                            linux_fd_set_descriptor_flags(newfd, LINUX_FD_CLOEXEC);
                        } else {
                            linux_fd_clear_descriptor_flags(newfd);
                        }
                        newfd as usize
                    }
                    Err(e) => linux_errno(e.code()),
                }
            }
            f::F_GETLK | f::F_OFD_GETLK => {
                if arg == 0 { return linux_fault(); }
                let mut flock: LinuxFlock = unsafe { core::mem::zeroed() };
                flock.l_type = f::F_UNLCK as i16;
                let _ = with_user_write_bytes(arg, core::mem::size_of::<LinuxFlock>(), |dst| {
                    let ptr = &flock as *const _ as *const u8;
                    dst.copy_from_slice(unsafe { core::slice::from_raw_parts(ptr, core::mem::size_of::<LinuxFlock>()) });
                    0
                });
                0
            }
            f::F_SETLK | f::F_SETLKW | f::F_OFD_SETLK | f::F_OFD_SETLKW => 0,
            f::F_GETOWN => crate::modules::posix::process::getpid() as usize,
            f::F_SETOWN => 0,
            f::F_GETPIPE_SZ => linux::PIPE_BUF_SIZE,
            f::F_SETPIPE_SZ => arg.max(4096).min(1 << 20).next_power_of_two(),
            _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn linux_fcntl_fd_flags_round_trip_and_validate_fd() {
        let fs_id =
            crate::modules::posix::fs::mount_ramfs("/linux-fcntl-fd-flags").expect("mount ramfs");
        let fd = crate::modules::posix::fs::creat(fs_id, "/roundtrip", 0o644).expect("creat");

        assert_eq!(sys_linux_fcntl(Fd(fd as i32), linux::fcntl::F_GETFD, 0), 0);
        assert_eq!(
            sys_linux_fcntl(Fd(fd as i32), linux::fcntl::F_SETFD, LINUX_FD_CLOEXEC),
            0
        );
        assert_eq!(
            sys_linux_fcntl(Fd(fd as i32), linux::fcntl::F_GETFD, 0),
            LINUX_FD_CLOEXEC
        );
        assert_eq!(
            crate::modules::posix::fs::fcntl_get_descriptor_flags(fd).expect("posix flags"),
            LINUX_FD_CLOEXEC as u32
        );
        assert_eq!(
            sys_linux_fcntl(Fd(424242), linux::fcntl::F_GETFD, 0),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );

        let _ = crate::modules::posix::fs::close(fd);
        let _ = crate::modules::posix::fs::unmount(fs_id);
    }
}

/// `ioctl(2)` — Control device.
pub fn sys_linux_ioctl(fd: Fd, cmd: usize, arg: usize) -> usize {
    let fd_val = fd.as_usize();

    // Framebuffer IOCTLs
    if fd_val == linux::FB_FD {
        if let Some(fb) = crate::hal::framebuffer() {
            match cmd {
                linux::FBIOGET_FSCREENINFO => {
                    let mut info: LinuxFbFixScreeninfo = unsafe { core::mem::zeroed() };
                    let id = b"hyperfb";
                    info.id[..id.len()].copy_from_slice(id);
                    info.smem_start = fb.address.as_ptr().unwrap() as u64;
                    info.smem_len = (fb.width * fb.height * (fb.bpp as u64 / 8)) as u32;
                    info.line_length = fb.pitch as u32;
                    let _ = with_user_write_bytes(
                        arg,
                        core::mem::size_of::<LinuxFbFixScreeninfo>(),
                        |dst| {
                            let ptr = &info as *const _ as *const u8;
                            dst.copy_from_slice(unsafe {
                                core::slice::from_raw_parts(
                                    ptr,
                                    core::mem::size_of::<LinuxFbFixScreeninfo>(),
                                )
                            });
                            0
                        },
                    );
                    return 0;
                }
                linux::FBIOGET_VSCREENINFO => {
                    let mut info: LinuxFbVarScreeninfo = unsafe { core::mem::zeroed() };
                    info.xres = fb.width as u32;
                    info.yres = fb.height as u32;
                    info.xres_virtual = fb.width as u32;
                    info.yres_virtual = fb.height as u32;
                    info.bits_per_pixel = fb.bpp as u32;
                    let _ = with_user_write_bytes(
                        arg,
                        core::mem::size_of::<LinuxFbVarScreeninfo>(),
                        |dst| {
                            let ptr = &info as *const _ as *const u8;
                            dst.copy_from_slice(unsafe {
                                core::slice::from_raw_parts(
                                    ptr,
                                    core::mem::size_of::<LinuxFbVarScreeninfo>(),
                                )
                            });
                            0
                        },
                    );
                    return 0;
                }
                _ => {}
            }
        }
    }

    // Terminal IOCTLs (TIOCGWINSZ etc)
    match cmd {
        TIOCGWINSZ => {
            let ws = LinuxWinsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            return with_user_write_bytes(arg, core::mem::size_of::<LinuxWinsize>(), |dst| {
                let ptr = &ws as *const _ as *const u8;
                dst.copy_from_slice(unsafe {
                    core::slice::from_raw_parts(ptr, core::mem::size_of::<LinuxWinsize>())
                });
                0
            })
            .map(|_| 0)
            .unwrap_or_else(|e| e);
        }
        _ => {}
    }

    // Generic VFS or Network IOCTLs
    crate::require_posix_io!((fd, cmd, arg) => {
        // Try network first
        #[cfg(feature = "network_transport")]
        {
            if let Some(ioctl_cmd) = crate::modules::libnet::PosixIoctlCmd::from_raw(cmd as u64) {
                if let Ok(res) = crate::modules::libnet::posix::ioctl(fd.as_u32(), ioctl_cmd) {
                    return res as usize;
                }
            }
        }

        // Then VFS
        match crate::modules::posix::fs::ioctl(fd.as_u32(), cmd as u32, arg as u64) {
            Ok(res) => res as usize,
            Err(_) => linux_errno(crate::modules::posix_consts::errno::ENOTTY),
        }
    })
}
