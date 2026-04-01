//! PTY (Pseudo-Terminal) subsystem for Linux compatibility.
//!
//! Implements /dev/ptmx (master allocator) and /dev/pts/* (slave terminals).
//! This is essential for SSH, tmux, screen, and interactive shells.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;
use spin::Mutex;

use crate::modules::vfs::types::{DirEntry, File, FileStats, PollEvents};
use super::dev_special::{Termios, WinSize, TCGETS, TCSETS, TCSETSW, TCSETSF};
use super::dev_special::{TIOCGPGRP, TIOCSPGRP, TIOCGWINSZ, TIOCSWINSZ, FIONREAD, FIONBIO};

const PTY_BUF_SIZE: usize = 4096;

// ── TIOCSPTLCK / TIOCGPTN ───────────────────────────────────────────────────
pub const TIOCSPTLCK: u32 = 0x40045431; // Lock/unlock PTY
pub const TIOCGPTN: u32 = 0x80045430;   // Get PTY number

// ── Shared PTY Pair State ───────────────────────────────────────────────────

struct PtyPairInner {
    /// Data written by master, read by slave (master→slave direction)
    master_to_slave: VecDeque<u8>,
    /// Data written by slave, read by master (slave→master direction)
    slave_to_master: VecDeque<u8>,
    /// PTY index number
    #[allow(dead_code)]
    index: u32,
    /// Locked state (TIOCSPTLCK)
    locked: bool,
    /// Terminal settings
    termios: Termios,
    /// Window size
    winsize: WinSize,
    /// Foreground process group
    fg_pgid: i32,
    /// Whether slave side has been opened
    slave_opened: bool,
    /// Closed flags
    master_closed: bool,
    slave_closed: bool,
}

#[derive(Clone)]
struct PtyPair(Arc<Mutex<PtyPairInner>>);

impl PtyPair {
    fn new(index: u32) -> Self {
        Self(Arc::new(Mutex::new(PtyPairInner {
            master_to_slave: VecDeque::with_capacity(PTY_BUF_SIZE),
            slave_to_master: VecDeque::with_capacity(PTY_BUF_SIZE),
            index,
            locked: true, // PTY starts locked
            termios: Termios::default(),
            winsize: WinSize::default(),
            fg_pgid: 0,
            slave_opened: false,
            master_closed: false,
            slave_closed: false,
        })))
    }
}

// ── Global PTY Registry ─────────────────────────────────────────────────────
#[path = "pty/registry.rs"]
mod registry;

pub use registry::init_pty_subsystem;

// ── PTY Master ──────────────────────────────────────────────────────────────

/// Master side of a PTY pair. Created when /dev/ptmx is opened.
pub struct PtyMaster {
    pair: PtyPair,
    index: u32,
}

impl PtyMaster {
    fn new(index: u32, pair: PtyPair) -> Self {
        Self { pair, index }
    }
}

impl File for PtyMaster {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut inner = self.pair.0.lock();
        if inner.slave_to_master.is_empty() {
            if inner.slave_closed {
                return Ok(0); // EOF
            }
            return Ok(0); // Would block (EAGAIN in non-blocking mode)
        }
        let mut count = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = inner.slave_to_master.pop_front() {
                *b = byte;
                count += 1;
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let mut inner = self.pair.0.lock();
        if inner.slave_closed {
            return Err("EIO");
        }
        // Master writes to the slave read buffer
        let _canonical = inner.termios.lflag & 0o000002 != 0; // ICANON

        for &byte in buf {
            // Input processing (ICRNL etc.)
            let processed = if inner.termios.iflag & 0o000400 != 0 && byte == b'\r' {
                b'\n' // ICRNL
            } else {
                byte
            };

            // Echo if enabled
            if inner.termios.lflag & 0o000010 != 0 {
                // ECHO
                inner.slave_to_master.push_back(processed);
            }

            if inner.master_to_slave.len() < PTY_BUF_SIZE {
                inner.master_to_slave.push_back(processed);
            }
        }
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match cmd {
            TIOCGPTN => {
                // Return PTY number
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *mut u32;
                unsafe { core::ptr::write_volatile(ptr, self.index) };
                Ok(0)
            }
            TIOCSPTLCK => {
                // Lock/unlock PTY
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const i32;
                let lock = unsafe { core::ptr::read_volatile(ptr) };
                let mut inner = self.pair.0.lock();
                inner.locked = lock != 0;
                Ok(0)
            }
            TCGETS => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut Termios;
                unsafe { core::ptr::write_volatile(ptr, inner.termios) };
                Ok(0)
            }
            TCSETS | TCSETSW | TCSETSF => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const Termios;
                let new_termios = unsafe { core::ptr::read_volatile(ptr) };
                let mut inner = self.pair.0.lock();
                inner.termios = new_termios;
                Ok(0)
            }
            TIOCGWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut WinSize;
                unsafe { core::ptr::write_volatile(ptr, inner.winsize) };
                Ok(0)
            }
            TIOCSWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const WinSize;
                let mut inner = self.pair.0.lock();
                inner.winsize = unsafe { core::ptr::read_volatile(ptr) };
                // TODO: Send SIGWINCH to slave process group
                Ok(0)
            }
            FIONREAD => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut i32;
                unsafe {
                    core::ptr::write_volatile(ptr, inner.slave_to_master.len() as i32);
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
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        let inner = self.pair.0.lock();
        let mut events = PollEvents::OUT;
        if !inner.slave_to_master.is_empty() {
            events |= PollEvents::IN;
        }
        if inner.slave_closed {
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
        let should_remove = {
            let mut inner = self.pair.0.lock();
            inner.master_closed = true;
            inner.master_closed && inner.slave_closed
        };
        if should_remove {
            registry::remove_pty(self.index);
        }
    }
}

// ── PTY Slave ───────────────────────────────────────────────────────────────

/// Slave side of a PTY pair. Opened via /dev/pts/N.
pub struct PtySlave {
    pair: PtyPair,
    index: u32,
}

impl PtySlave {
    fn new(index: u32, pair: PtyPair) -> Self {
        Self { pair, index }
    }
}

impl File for PtySlave {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut inner = self.pair.0.lock();
        if inner.master_to_slave.is_empty() {
            if inner.master_closed {
                return Ok(0); // EOF
            }
            return Ok(0); // EAGAIN
        }

        let canonical = inner.termios.lflag & 0o000002 != 0; // ICANON
        let mut count = 0;

        for b in buf.iter_mut() {
            if let Some(byte) = inner.master_to_slave.pop_front() {
                *b = byte;
                count += 1;
                // In canonical mode, return after newline
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
        let mut inner = self.pair.0.lock();
        if inner.master_closed {
            return Err("EIO");
        }

        for &byte in buf {
            // Output processing (OPOST + ONLCR: \n → \r\n)
            if inner.termios.oflag & 0o000001 != 0 && inner.termios.oflag & 0o000004 != 0 {
                if byte == b'\n' {
                    if inner.slave_to_master.len() < PTY_BUF_SIZE {
                        inner.slave_to_master.push_back(b'\r');
                    }
                }
            }
            if inner.slave_to_master.len() < PTY_BUF_SIZE {
                inner.slave_to_master.push_back(byte);
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
                let inner = self.pair.0.lock();
                let ptr = arg as *mut Termios;
                unsafe { core::ptr::write_volatile(ptr, inner.termios) };
                Ok(0)
            }
            TCSETS | TCSETSW | TCSETSF => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const Termios;
                let new_termios = unsafe { core::ptr::read_volatile(ptr) };
                let mut inner = self.pair.0.lock();
                inner.termios = new_termios;
                Ok(0)
            }
            TIOCGWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut WinSize;
                unsafe { core::ptr::write_volatile(ptr, inner.winsize) };
                Ok(0)
            }
            TIOCSWINSZ => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const WinSize;
                let mut inner = self.pair.0.lock();
                inner.winsize = unsafe { core::ptr::read_volatile(ptr) };
                Ok(0)
            }
            TIOCGPGRP => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut i32;
                unsafe { core::ptr::write_volatile(ptr, inner.fg_pgid) };
                Ok(0)
            }
            TIOCSPGRP => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let ptr = arg as *const i32;
                let mut inner = self.pair.0.lock();
                inner.fg_pgid = unsafe { core::ptr::read_volatile(ptr) };
                Ok(0)
            }
            FIONREAD => {
                if arg == 0 {
                    return Err("EFAULT");
                }
                let inner = self.pair.0.lock();
                let ptr = arg as *mut i32;
                unsafe {
                    core::ptr::write_volatile(ptr, inner.master_to_slave.len() as i32);
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
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        let inner = self.pair.0.lock();
        let mut events = PollEvents::OUT;
        if !inner.master_to_slave.is_empty() {
            events |= PollEvents::IN;
        }
        if inner.master_closed {
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
        let should_remove = {
            let mut inner = self.pair.0.lock();
            inner.slave_closed = true;
            inner.master_closed && inner.slave_closed
        };
        if should_remove {
            registry::remove_pty(self.index);
        }
    }
}

#[path = "pty/dev.rs"]
mod dev;
#[path = "pty/fs.rs"]
mod fs;

pub use dev::{open_ptmx, open_pts, DevPtmx};
pub use fs::PtsFs;
