use crate::modules::vfs::dev_special::{
    FIONBIO, FIONREAD, TCGETS, TCSETS, TCSETSF, TCSETSW, TIOCGPGRP, TIOCGWINSZ, TIOCNOTTY,
    TIOCSPGRP, TIOCSCTTY, TIOCSWINSZ, Termios, WinSize,
};

use super::pair::PtyPair;

pub(crate) enum PtyIoctlSide {
    Master,
    Slave,
}

pub(crate) fn handle_common_ioctl(
    pair: &PtyPair,
    side: PtyIoctlSide,
    cmd: u32,
    arg: u64,
) -> Result<Option<isize>, &'static str> {
    let result = match cmd {
        TCGETS => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *mut Termios;
            unsafe { core::ptr::write_volatile(ptr, pair.get_termios()) };
            Some(0)
        }
        TCSETS | TCSETSW | TCSETSF => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *const Termios;
            let termios = unsafe { core::ptr::read_volatile(ptr) };
            pair.set_termios(termios);
            Some(0)
        }
        TIOCGWINSZ => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *mut WinSize;
            unsafe { core::ptr::write_volatile(ptr, pair.get_winsize()) };
            Some(0)
        }
        TIOCSWINSZ => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *const WinSize;
            let winsize = unsafe { core::ptr::read_volatile(ptr) };
            let fg_pgid = pair.set_winsize(winsize);
            pair.maybe_signal_foreground_group(fg_pgid);
            Some(0)
        }
        TIOCGPGRP => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *mut i32;
            unsafe { core::ptr::write_volatile(ptr, pair.fg_pgid()) };
            Some(0)
        }
        TIOCSPGRP => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *const i32;
            let fg_pgid = unsafe { core::ptr::read_volatile(ptr) };
            if fg_pgid <= 0 {
                return Err("EINVAL");
            }
            pair.set_fg_pgid(fg_pgid);
            Some(0)
        }
        TIOCSCTTY => {
            #[cfg(feature = "posix_process")]
            {
                let pid = crate::modules::posix::process::getpid();
                if pid == 0 {
                    return Err("EPERM");
                }

                let sid = crate::modules::posix::process::getsid(0).map_err(|_| "EPERM")?;
                if sid != pid {
                    return Err("EPERM");
                }

                let pgrp = crate::modules::posix::process::getpgrp() as i32;
                pair.attach_controlling_session(sid, pgrp)?;
                Some(0)
            }

            #[cfg(not(feature = "posix_process"))]
            {
                let _ = arg;
                return Err("ENOTTY");
            }
        }
        TIOCNOTTY => {
            #[cfg(feature = "posix_process")]
            {
                let sid = crate::modules::posix::process::getsid(0).map_err(|_| "EPERM")?;
                pair.detach_controlling_session(sid)?;
                Some(0)
            }

            #[cfg(not(feature = "posix_process"))]
            {
                let _ = arg;
                return Err("ENOTTY");
            }
        }
        FIONREAD => {
            if arg == 0 {
                return Err("EFAULT");
            }
            let ptr = arg as *mut i32;
            let count = match side {
                PtyIoctlSide::Master => pair.slave_to_master_len(),
                PtyIoctlSide::Slave => pair.master_to_slave_len(),
            };
            unsafe { core::ptr::write_volatile(ptr, count as i32) };
            Some(0)
        }
        FIONBIO => Some(0),
        _ => None,
    };

    Ok(result)
}
