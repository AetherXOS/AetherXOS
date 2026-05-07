use core::any::Any;

use crate::modules::vfs::types::{File, FileStats, PollEvents};

use super::ioctl::{handle_common_ioctl, PtyIoctlSide};
use super::pair::PtyPair;

pub(crate) struct PtySlave {
    pair: PtyPair,
    index: u32,
}

impl PtySlave {
    pub(crate) fn new(index: u32, pair: PtyPair) -> Self {
        Self { pair, index }
    }
}

impl File for PtySlave {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pair.master_to_slave_len() == 0 {
            if self.pair.master_closed() {
                return Ok(0);
            }
            return Ok(0);
        }

        let canonical = self.pair.get_termios().lflag & 0o000002 != 0;
        let mut count = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.pair.pop_master_to_slave() {
                *b = byte;
                count += 1;
                if canonical && byte == b'\n' {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if self.pair.master_closed() {
            return Err("EIO");
        }

        let termios = self.pair.get_termios();
        for &byte in buf {
            if termios.oflag & 0o000001 != 0 && termios.oflag & 0o000004 != 0 && byte == b'\n' {
                let _ = self.pair.push_slave_to_master(b'\r');
            }
            let _ = self.pair.push_slave_to_master(byte);
        }
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match handle_common_ioctl(&self.pair, PtyIoctlSide::Slave, cmd, arg)? {
            Some(result) => Ok(result),
            None => Err("ENOTTY"),
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
        if self.pair.master_to_slave_len() > 0 {
            events |= PollEvents::IN;
        }
        if self.pair.master_closed() {
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

impl Drop for PtySlave {
    fn drop(&mut self) {
        if self.pair.mark_slave_closed() {
            super::registry::remove_pty(self.index);
        }
    }
}
