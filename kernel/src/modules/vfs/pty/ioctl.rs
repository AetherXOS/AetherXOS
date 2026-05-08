use crate::modules::vfs::dev_special::{
    FIONBIO, FIONREAD, TCGETS, TCSETS, TCSETSF, TCSETSW, TIOCGPGRP, TIOCGWINSZ, TIOCNOTTY,
    TIOCSPGRP, TIOCSCTTY, TIOCSWINSZ, Termios, WinSize,
};

use super::pair::PtyPair;

pub(crate) enum PtyIoctlSide {
    Master,
    Slave,
}

macro_rules! ioctl_write {
    ($arg:expr, $ty:ty, $value:expr) => {{
        if $arg == 0 {
            return Err("EFAULT");
        }
        let ptr = $arg as *mut $ty;
        unsafe { core::ptr::write_volatile(ptr, $value) };
        Some(0)
    }};
}

macro_rules! ioctl_read {
    ($arg:expr, $ty:ty, $handler:expr) => {{
        if $arg == 0 {
            return Err("EFAULT");
        }
        let ptr = $arg as *const $ty;
        let value = unsafe { core::ptr::read_volatile(ptr) };
        $handler(value)
    }};
}

pub(crate) fn handle_common_ioctl(
    pair: &PtyPair,
    side: PtyIoctlSide,
    cmd: u32,
    arg: u64,
) -> Result<Option<isize>, &'static str> {
    let result = match cmd {
        TCGETS => ioctl_write!(arg, Termios, pair.get_termios()),
        TCSETS | TCSETSW | TCSETSF => ioctl_read!(arg, Termios, |termios| {
            pair.set_termios(termios);
            Some(0)
        }),
        TIOCGWINSZ => ioctl_write!(arg, WinSize, pair.get_winsize()),
        TIOCSWINSZ => ioctl_read!(arg, WinSize, |winsize| {
            let fg_pgid = pair.set_winsize(winsize);
            pair.maybe_signal_foreground_group(fg_pgid);
            Some(0)
        }),
        TIOCGPGRP => ioctl_write!(arg, i32, pair.fg_pgid()),
        TIOCSPGRP => ioctl_read!(arg, i32, |fg_pgid| {
            if fg_pgid <= 0 {
                return Err("EINVAL");
            }
            pair.set_fg_pgid(fg_pgid);
            Some(0)
        }),
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
        FIONREAD => ioctl_write!(arg, i32, match side {
            PtyIoctlSide::Master => pair.slave_to_master_len(),
            PtyIoctlSide::Slave => pair.master_to_slave_len(),
        } as i32),
        FIONBIO => Some(0),
        _ => None,
    };

    Ok(result)
}
