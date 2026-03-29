//! Special device file implementations for Linux compatibility.
//!
//! Provides proper /dev/null, /dev/zero, /dev/full, /dev/random, /dev/urandom,
//! /dev/tty, /dev/console, /dev/stdin, /dev/stdout, /dev/stderr devices.

extern crate alloc;

use alloc::boxed::Box;
use core::any::Any;
use crate::interfaces::hardware::SerialDevice;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{File, FileStats, PollEvents, SeekFrom};

#[path = "dev_special/stdio.rs"]
mod stdio;
pub use stdio::register_linux_special_devices;

// ── PRNG State ──────────────────────────────────────────────────────────────

/// Global PRNG state seeded from RDRAND or TSC at boot.
static PRNG_STATE: AtomicU64 = AtomicU64::new(0x6A09E667F3BCC908);

/// Mix function based on SplitMix64 — fast, decent quality for /dev/urandom.
#[inline(always)]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Seed the PRNG from hardware entropy (call during init).
pub fn seed_prng(entropy: u64) {
    PRNG_STATE.store(entropy, Ordering::Relaxed);
}

/// Fill a buffer with pseudo-random bytes.
fn fill_random_bytes(buf: &mut [u8]) {
    let mut state = PRNG_STATE.load(Ordering::Relaxed);
    state ^= crate::hal::cpu::rdtsc();

    let mut pos = 0;
    while pos < buf.len() {
        let word = splitmix64(&mut state);
        let bytes = word.to_le_bytes();
        let remaining = buf.len() - pos;
        let copy_len = remaining.min(8);
        buf[pos..pos + copy_len].copy_from_slice(&bytes[..copy_len]);
        pos += copy_len;
    }
    PRNG_STATE.store(state, Ordering::Relaxed);
}

// ── /dev/null ────────────────────────────────────────────────────────────────

/// `/dev/null` — reads return EOF, writes succeed silently.
pub struct DevNull;

impl File for DevNull {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(0) // EOF
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len()) // discard
    }
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Ok(0)
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666, // char device, rw-rw-rw-
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::OUT // always writable
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── /dev/zero ────────────────────────────────────────────────────────────────

/// `/dev/zero` — reads return zeroes, writes succeed silently.
pub struct DevZero;

impl File for DevZero {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Ok(0)
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::IN | PollEvents::OUT
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── /dev/full ────────────────────────────────────────────────────────────────

/// `/dev/full` — reads return zeroes, writes fail with ENOSPC.
pub struct DevFull;

impl File for DevFull {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("ENOSPC")
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::IN
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── /dev/random & /dev/urandom ───────────────────────────────────────────────

/// `/dev/random` and `/dev/urandom` — reads return pseudo-random bytes.
/// In modern Linux (5.6+), both are equivalent. We follow this model.
pub struct DevRandom;

impl File for DevRandom {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        fill_random_bytes(buf);
        Ok(buf.len())
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        // Writing to /dev/random mixes entropy (best-effort: XOR into state)
        if !buf.is_empty() {
            let mut mix: u64 = 0;
            for (i, &b) in buf.iter().enumerate() {
                mix ^= (b as u64) << ((i % 8) * 8);
            }
            let old = PRNG_STATE.load(Ordering::Relaxed);
            PRNG_STATE.store(old ^ mix, Ordering::Relaxed);
        }
        Ok(buf.len())
    }
    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020444, // char device, r--r--r--
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::IN | PollEvents::OUT
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── /dev/tty & /dev/console ──────────────────────────────────────────────────

/// Terminal line discipline flags.
#[derive(Debug, Clone, Copy)]
pub struct Termios {
    pub iflag: u32,  // input modes
    pub oflag: u32,  // output modes
    pub cflag: u32,  // control modes
    pub lflag: u32,  // local modes
    pub cc: [u8; 32], // control characters
}

impl Default for Termios {
    fn default() -> Self {
        let mut cc = [0u8; 32];
        // Standard control characters
        cc[0] = 0x03;  // VINTR = Ctrl-C
        cc[1] = 0x1C;  // VQUIT = Ctrl-backslash
        cc[2] = 0x7F;  // VERASE = DEL
        cc[3] = 0x15;  // VKILL = Ctrl-U
        cc[4] = 0x04;  // VEOF = Ctrl-D
        cc[5] = 0x00;  // VTIME
        cc[6] = 0x01;  // VMIN
        cc[7] = 0x00;  // VSWTC
        cc[8] = 0x11;  // VSTART = Ctrl-Q
        cc[9] = 0x13;  // VSTOP = Ctrl-S
        cc[10] = 0x1A; // VSUSP = Ctrl-Z
        cc[11] = 0x00; // VEOL
        cc[12] = 0x12; // VREPRINT = Ctrl-R
        cc[13] = 0x0F; // VDISCARD = Ctrl-O
        cc[14] = 0x17; // VWERASE = Ctrl-W
        cc[15] = 0x16; // VLNEXT = Ctrl-V
        cc[16] = 0x00; // VEOL2

        Self {
            iflag: 0o013602, // ICRNL | IXON | IXOFF | IUTF8
            oflag: 0o000005, // OPOST | ONLCR
            cflag: 0o000277, // B38400 | CS8 | CREAD
            lflag: 0o105073, // ISIG | ICANON | ECHO | ECHOE | ECHOK | ECHOCTL | ECHOKE | IEXTEN
            cc,
        }
    }
}

// TTY ioctl commands
pub const TCGETS: u32 = 0x5401;
pub const TCSETS: u32 = 0x5402;
pub const TCSETSW: u32 = 0x5403;
pub const TCSETSF: u32 = 0x5404;
pub const TIOCGPGRP: u32 = 0x540F;
pub const TIOCSPGRP: u32 = 0x5410;
pub const TIOCGWINSZ: u32 = 0x5413;
pub const TIOCSWINSZ: u32 = 0x5414;
pub const TIOCSCTTY: u32 = 0x540E;
pub const TIOCNOTTY: u32 = 0x5422;
pub const FIONREAD: u32 = 0x541B;
pub const FIONBIO: u32 = 0x5421;

/// Window size structure matching Linux `struct winsize`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

impl Default for WinSize {
    fn default() -> Self {
        Self {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

/// `/dev/tty` — basic terminal device.
/// This provides a serial-console backed TTY with termios support.
pub struct DevTty {
    termios: Termios,
    winsize: WinSize,
    fg_pgid: i32,
    input_buf: alloc::collections::VecDeque<u8>,
}

impl DevTty {
    pub fn new() -> Self {
        Self {
            termios: Termios::default(),
            winsize: WinSize::default(),
            fg_pgid: 1,
            input_buf: alloc::collections::VecDeque::with_capacity(4096),
        }
    }
}

impl File for DevTty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        // Try to read from serial/input buffer
        if self.input_buf.is_empty() {
            // Non-blocking: return 0 (EAGAIN)
            return Ok(0);
        }
        let mut count = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.input_buf.pop_front() {
                *b = byte;
                count += 1;
                // In canonical mode, stop at newline
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
        // Write to serial output
        for &byte in buf {
            // OPOST + ONLCR: convert \n to \r\n
            if self.termios.oflag & 0o000004 != 0 && byte == b'\n' {
                #[cfg(target_arch = "x86_64")]
                {
                    crate::hal::x86_64::serial::SERIAL1.lock().send(b'\r');
                }
            }
            #[cfg(target_arch = "x86_64")]
            {
                crate::hal::x86_64::serial::SERIAL1.lock().send(byte);
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                let _ = byte; // suppress unused warning on non-x86
            }
        }
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match cmd {
            TCGETS => {
                // Return termios structure
                if arg == 0 {
                    return Err("EFAULT");
                }
                // Write termios to user space
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
                self.winsize = unsafe { core::ptr::read_volatile(ptr) };
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
                self.fg_pgid = unsafe { core::ptr::read_volatile(ptr) };
                Ok(0)
            }
            TIOCSCTTY | TIOCNOTTY => Ok(0), // stub: success
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
            FIONBIO => Ok(0), // stub: non-blocking mode toggle
            _ => Err("ENOTTY"),
        }
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020620, // char device, rw--w----
            uid: 0,
            gid: 5, // tty group
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
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

