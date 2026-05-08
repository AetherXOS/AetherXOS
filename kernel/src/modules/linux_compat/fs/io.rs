use super::super::*;
use crate::kernel::syscalls::with_user_write_bytes;
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

/// FD_CLOEXEC flag value in Linux
pub(crate) const LINUX_FD_CLOEXEC: usize = 0x1;

// Terminal ioctl command codes (from <asm/ioctls.h>)
const TIOCGWINSZ:  usize = 0x5413; // Get terminal window size
const TIOCSWINSZ:  usize = 0x5414; // Set terminal window size
const TIOCGPGRP:   usize = 0x540F; // Get foreground process group
const TIOCSPGRP:   usize = 0x5410; // Set foreground process group
const TCGETS:      usize = 0x5401; // Get termios
const TCSETS:      usize = 0x5402; // Set termios (immediate)
const TCSETSW:     usize = 0x5403; // Set termios (drain first)
const TCSETSF:     usize = 0x5404; // Set termios (flush first)
const TIOCSCTTY:   usize = 0x540E; // Set controlling terminal
const TIOCGPTN:    usize = 0x8004_5430; // Get pty number
const TIOCSPTLCK:  usize = 0x4004_5431; // Lock/unlock pty
const TIOCGSERIAL: usize = 0x541E; // Get serial settings (stub)
const FIONREAD:    usize = 0x541B; // Get bytes available to read
const FIONBIO:     usize = 0x5421; // Set non-blocking mode
const TIOCNOTTY:   usize = 0x5422; // Detach from controlling terminal

// Pipe and buffer size constants
const PIPE_MIN_SIZE: usize = 4096;
const PIPE_MAX_SIZE: usize = 1 << 20; // 1 MB

// Terminal constants
const TERMINAL_DEFAULT_ROWS: u16 = 24;
const TERMINAL_DEFAULT_COLS: u16 = 80;

lazy_static! {
    static ref LINUX_FD_FLAGS: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());
}

#[allow(dead_code)]
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
    let entries: alloc::vec::Vec<(u32, bool)> = {
        let table = crate::modules::posix::fs::FILE_TABLE.lock();
        table.iter().map(|(fd, desc)| (*fd, desc.cloexec)).collect()
    };
    let linux_flags = LINUX_FD_FLAGS.lock().clone();
    let fds: alloc::vec::Vec<u32> = entries
        .into_iter()
        .filter_map(|(fd, cloexec)| {
            let linux_flags = linux_flags.get(&fd).copied().unwrap_or(0);
            if cloexec || (linux_flags & LINUX_FD_CLOEXEC) != 0 {
                Some(fd)
            } else {
                None
            }
        })
        .collect();

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
                match crate::modules::posix::fs::fcntl_get_status_flags(fd.as_u32()) {
                    Ok(flags) => flags as usize,
                    Err(e) => linux_errno(e.code()),
                }
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
            f::F_SETPIPE_SZ => arg.max(PIPE_MIN_SIZE).min(PIPE_MAX_SIZE).next_power_of_two(),
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
        assert_eq!(
            sys_linux_fcntl(Fd(424242), linux::fcntl::F_GETFL, 0),
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
                    info.smem_start = match fb.address.as_ptr() {
                        Some(ptr) => ptr as u64,
                        None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
                    };
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

    // Terminal IOCTLs
    match cmd {
        TIOCGWINSZ => {
            let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
            let ws = if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
                let kws = tty.get_winsize();
                LinuxWinsize {
                    ws_row: kws.ws_row,
                    ws_col: kws.ws_col,
                    ws_xpixel: kws.ws_xpixel,
                    ws_ypixel: kws.ws_ypixel,
                }
            } else {
                LinuxWinsize {
                    ws_row: TERMINAL_DEFAULT_ROWS,
                    ws_col: TERMINAL_DEFAULT_COLS,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                }
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
        TIOCSWINSZ => {
            if arg == 0 { return linux_fault(); }
            let mut ws: LinuxWinsize = unsafe { core::mem::zeroed() };
            let _ = crate::kernel::syscalls::with_user_read_bytes(arg, core::mem::size_of::<LinuxWinsize>(), |src| {
                let ptr = &mut ws as *mut _ as *mut u8;
                unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), ptr, core::mem::size_of::<LinuxWinsize>()); }
                0
            });
            let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
            if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
                tty.set_winsize(crate::kernel::tty::WinSize {
                    ws_row: ws.ws_row,
                    ws_col: ws.ws_col,
                    ws_xpixel: ws.ws_xpixel,
                    ws_ypixel: ws.ws_ypixel,
                });
            }
            return 0;
        }
        TIOCGPGRP => {
            return match crate::modules::linux_compat::process_group_syscalls::sys_ioctl_tiocgpgrp(fd_val, arg) {
                Ok(pgrp) => {
                    let _ = crate::kernel::syscalls::write_user_pod(arg, &(pgrp as i32));
                    0
                }
                Err(_) => linux_errno(crate::modules::posix_consts::errno::ENOTTY),
            };
        }
        TIOCSPGRP => {
            let mut pgrp: i32 = 0;
            if let Err(e) = crate::kernel::syscalls::read_user_pod(arg, &mut pgrp) { return e; }
            return match crate::modules::linux_compat::process_group_syscalls::sys_ioctl_tiocspgrp(fd_val, pgrp as usize) {
                Ok(()) => 0,
                Err(_) => linux_errno(crate::modules::posix_consts::errno::ENOTTY),
            };
        }
        TCGETS => {
            let termios = LinuxTermios::default();
            return with_user_write_bytes(arg, core::mem::size_of::<LinuxTermios>(), |dst| {
                let ptr = &termios as *const _ as *const u8;
                dst.copy_from_slice(unsafe {
                    core::slice::from_raw_parts(ptr, core::mem::size_of::<LinuxTermios>())
                });
                0
            }).map(|_| 0).unwrap_or_else(|e| e);
        }
        TCSETS | TCSETSW | TCSETSF => {
            // Accept and apply - update the TTY device's termios
            if fd_val <= 2 {
                let mut termios = LinuxTermios::default();
                if let Err(e) = with_user_read_bytes(arg, core::mem::size_of::<LinuxTermios>(), |src| {
                    let ptr = &mut termios as *mut _ as *mut u8;
                    unsafe {
                        ptr.copy_from_nonoverlapping(src.as_ptr(), core::mem::size_of::<LinuxTermios>());
                    }
                    Ok(())
                }) {
                    return e;
                }
                let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
                if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
                    tty.set_termios(termios);
                }
            }
            return 0;
        }
        TIOCSCTTY => {
            // Associate this TTY with current session
            // arg = 1 means steal from another session
            let registry = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock();
            if let Some(tty) = registry.get(crate::kernel::tty::TtyId::new(0)) {
                #[cfg(feature = "posix_process")]
                {
                    let sid = crate::modules::posix::process::getsid(0).unwrap_or(0);
                    tty.set_session_id(Some(crate::kernel::tty::SessionId(
                        crate::interfaces::task::ProcessId(sid)
                    )));
                }
            }
            return 0;
        }
        TIOCNOTTY => {
            // Detach from controlling terminal
            return 0;
        }
        FIONREAD => {
            if arg == 0 { return linux_fault(); }
            // For now return 0 bytes available (non-blocking hint)
            let available: i32 = 0;
            let _ = crate::kernel::syscalls::write_user_pod(arg, &available);
            return 0;
        }
        FIONBIO => {
            // Set/clear non-blocking mode on fd
            if arg != 0 {
                let flag: i32 = if unsafe { *(arg as *const i32) } != 0 { 1 } else { 0 };
                #[cfg(feature = "posix_fs")]
                {
                    let mut flags = crate::modules::posix::fs::fcntl_get_status_flags(fd.as_u32())
                        .unwrap_or(0);
                    if flag != 0 {
                        flags |= crate::modules::posix_consts::fs::O_NONBLOCK as u32;
                    } else {
                        flags &= !(crate::modules::posix_consts::fs::O_NONBLOCK as u32);
                    }
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(fd.as_u32(), flags);
                }
            }
            return 0;
        }
        TIOCGPTN => {
            // Get PTY number - return 0 for now (pts/0)
            if arg == 0 { return linux_fault(); }
            let pty_num: u32 = 0;
            let _ = crate::kernel::syscalls::write_user_pod(arg, &pty_num);
            return 0;
        }
        TIOCSPTLCK => {
            // Lock/unlock PTY - accept and ignore
            return 0;
        }
        TIOCGSERIAL => {
            // Serial settings - return ENOTTY for non-serial fds (correct behavior)
            return linux_errno(crate::modules::posix_consts::errno::ENOTTY);
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
