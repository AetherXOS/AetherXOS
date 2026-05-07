use core::any::Any;

use crate::modules::vfs::types::{File, FileStats, PollEvents};

use super::ioctl::{handle_common_ioctl, PtyIoctlSide};
use super::pair::PtyPair;
use super::{TIOCGPTN, TIOCSPTLCK};

pub(crate) struct PtyMaster {
    pair: PtyPair,
    index: u32,
}

impl PtyMaster {
    pub(crate) fn new(index: u32, pair: PtyPair) -> Self {
        Self { pair, index }
    }
}

impl File for PtyMaster {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pair.slave_to_master_len() == 0 {
            if self.pair.slave_closed() {
                return Ok(0);
            }
            return Ok(0);
        }

        let mut count = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.pair.pop_slave_to_master() {
                *b = byte;
                count += 1;
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if self.pair.slave_closed() {
            return Err("EIO");
        }

        let termios = self.pair.get_termios();
        for &byte in buf {
            let processed = if termios.iflag & 0o000400 != 0 && byte == b'\r' {
                b'\n'
            } else {
                byte
            };

            if termios.lflag & 0o000010 != 0 {
                let _ = self.pair.push_slave_to_master(processed);
            }

            let _ = self.pair.push_master_to_slave(processed);
        }
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match cmd {
            TIOCGPTN => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut u32;
                unsafe { core::ptr::write_volatile(ptr, self.index) };
                Ok(0)
            }
            TIOCSPTLCK => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const i32;
                let lock = unsafe { core::ptr::read_volatile(ptr) };
                self.pair.set_locked(lock != 0);
                Ok(0)
            }
            _ => match handle_common_ioctl(&self.pair, PtyIoctlSide::Master, cmd, arg)? {
                Some(result) => Ok(result),
                None => Err("ENOTTY"),
            },
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
            blocks: 0,
            ..Default::default()
        })
    }

    fn poll_events(&self) -> PollEvents {
        let mut events = PollEvents::OUT;
        if self.pair.slave_to_master_len() > 0 {
            events |= PollEvents::IN;
        }
        if self.pair.slave_closed() {
            events |= PollEvents::HUP;
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

impl Drop for PtyMaster {
    fn drop(&mut self) {
        if self.pair.mark_master_closed() {
            super::registry::remove_pty(self.index);
        }
    }
}
