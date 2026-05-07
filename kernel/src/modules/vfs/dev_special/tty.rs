extern crate alloc;

use alloc::collections::VecDeque;
use core::any::Any;
use crate::interfaces::task::ProcessId;
use crate::interfaces::hardware::SerialDevice;
use crate::kernel::tty::{GLOBAL_TTY_REGISTRY, ProcessGroupId, SessionId, TtyId};
use crate::modules::vfs::types::{File, FileStats, PollEvents};
use super::termios::{Termios, WinSize, TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGWINSZ, TIOCSWINSZ, TIOCGPGRP, TIOCSPGRP, TIOCSCTTY, TIOCNOTTY, FIONREAD, FIONBIO};

/// `/dev/tty` — basic terminal device.
pub struct DevTty {
    termios: Termios,
    winsize: WinSize,
    fg_pgid: i32,
    input_buf: VecDeque<u8>,
}

impl DevTty {
    pub fn new() -> Self {
        Self {
            termios: Termios::default(),
            winsize: WinSize::default(),
            fg_pgid: 1,
            input_buf: VecDeque::with_capacity(4096),
        }
    }
}

impl File for DevTty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.input_buf.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.input_buf.pop_front() {
                *b = byte;
                count += 1;
                if self.termios.lflag & 0o000002 != 0 && byte == b'\n' {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        for &byte in buf {
            if self.termios.oflag & 0o000004 != 0 && byte == b'\n' {
                #[cfg(target_arch = "x86_64")]
                {
                    crate::hal::serial::SERIAL1.lock().send(b'\r');
                }
            }
            #[cfg(target_arch = "x86_64")]
            {
                crate::hal::serial::SERIAL1.lock().send(byte);
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                let _ = byte;
            }
        }
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match cmd {
            TCGETS => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut Termios;
                unsafe { core::ptr::write_volatile(ptr, self.termios) };
                Ok(0)
            }
            TCSETS | TCSETSW | TCSETSF => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const Termios;
                let new_termios = unsafe { core::ptr::read_volatile(ptr) };
                self.termios = new_termios;
                Ok(0)
            }
            TIOCGWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut WinSize;
                unsafe { core::ptr::write_volatile(ptr, self.winsize) };
                Ok(0)
            }
            TIOCSWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const WinSize;
                let new_winsize = unsafe { core::ptr::read_volatile(ptr) };
                self.winsize = new_winsize;

                // Sync to global TTY and signal foreground group
                let registry = GLOBAL_TTY_REGISTRY.lock();
                if let Some(tty) = registry.get(TtyId::new(0)) {
                    tty.set_winsize(new_winsize);
                    if let Some(fg_pgrp) = tty.foreground_pgrp() {
                        let sigwinch = crate::modules::posix_consts::signal::SIGWINCH;
                        let _ = crate::modules::posix::process::killpg((fg_pgrp.0).0, sigwinch);
                    }
                }
                Ok(0)
            }
            TIOCGPGRP => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut i32;
                unsafe { core::ptr::write_volatile(ptr, self.fg_pgid) };
                Ok(0)
            }
            TIOCSPGRP => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const i32;
                let new_fg_pgid = unsafe { core::ptr::read_volatile(ptr) };
                self.fg_pgid = new_fg_pgid;

                // Sync to global TTY
                let registry = GLOBAL_TTY_REGISTRY.lock();
                if let Some(tty) = registry.get(TtyId::new(0)) {
                    tty.set_foreground_pgrp(Some(ProcessGroupId(ProcessId(new_fg_pgid as usize))));
                }
                Ok(0)
            }
            TIOCSCTTY => {
                let pid = crate::modules::posix::process::getpid();
                if pid == 0 {
                    return Err("EPERM");
                }

                let sid = match crate::modules::posix::process::getsid(0) {
                    Ok(value) => value,
                    Err(_) => return Err("EPERM"),
                };
                if sid != pid {
                    return Err("EPERM");
                }

                let pgid = crate::modules::posix::process::getpgrp();
                let registry = GLOBAL_TTY_REGISTRY.lock();
                let tty = match registry.get(TtyId::new(0)) {
                    Some(tty) => tty,
                    None => return Err("ENOTTY"),
                };

                if arg == 0 {
                    if let Some(existing_sid) = tty.session_id() {
                        if (existing_sid.0).0 != sid {
                            return Err("EBUSY");
                        }
                    }
                }

                tty.set_session_id(Some(SessionId(ProcessId(sid))));
                tty.set_foreground_pgrp(Some(ProcessGroupId(ProcessId(pgid))));
                Ok(0)
            }
            TIOCNOTTY => {
                let pid = crate::modules::posix::process::getpid();
                if pid == 0 {
                    return Err("EPERM");
                }

                let sid = match crate::modules::posix::process::getsid(0) {
                    Ok(value) => value,
                    Err(_) => return Err("EPERM"),
                };

                let registry = GLOBAL_TTY_REGISTRY.lock();
                let tty = match registry.get(TtyId::new(0)) {
                    Some(tty) => tty,
                    None => return Err("ENOTTY"),
                };

                if let Some(existing_sid) = tty.session_id() {
                    if (existing_sid.0).0 != sid {
                        return Err("EPERM");
                    }
                }

                tty.set_session_id(None);
                tty.set_foreground_pgrp(None);
                Ok(0)
            }
            FIONREAD => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut i32;
                unsafe {
                    core::ptr::write_volatile(ptr, self.input_buf.len() as i32);
                }
                Ok(0)
            }
            FIONBIO => Ok(0),
            _ => Err("ENOTTY"),
        }
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020620,
            uid: 0,
            gid: 5,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,
            ..crate::modules::vfs::types::FileStats::default()
        })
    }

    fn poll_events(&self) -> PollEvents {
        let mut events = PollEvents::OUT;
        if !self.input_buf.is_empty() {
            events |= PollEvents::IN;
        }
        events
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
