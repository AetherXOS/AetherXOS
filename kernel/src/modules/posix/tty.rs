use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;
use crate::modules::vfs::types::{File, FileStats, FileType};

/// termios structure matching Linux ABI
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

/// Window size structure for TIOCGWINSZ
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

use crate::kernel::sync::ring_buffer::RingBuffer;

pub struct Tty {
    pub name: String,
    pub termios: Mutex<Termios>,
    pub winsize: Mutex<WinSize>,
    pub buffer: RingBuffer<u8>,
}

impl Tty {
    pub fn new(name: String) -> Self {
        let mut termios = Termios::default();
        termios.c_lflag = 0x02 | 0x08; // ECHO | ICANON
        
        Self {
            name,
            termios: Mutex::new(termios),
            winsize: Mutex::new(WinSize::default()),
            buffer: RingBuffer::new(4096),
        }
    }

    pub fn push_byte(&self, byte: u8) {
        let lflag = self.termios.lock().c_lflag;
        if (lflag & 0x08) != 0 {
            crate::klog_info!("[TTY ECHO] {}", byte as char);
        }

        let _ = self.buffer.push(byte);
    }
}

impl File for Tty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut count = 0;
        while count < buf.len() {
            if let Some(b) = self.buffer.pop() {
                buf[count] = b;
                count += 1;
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        // Line Discipline: Echo back to input for testing or send to screen
        // In a real PTY, this would go to the master side.
        crate::klog_info!("[TTY] Write: {:?}", buf);
        Ok(buf.len())
    }

    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        match cmd {
            0x5401 => { // TCGETS
                let ptr = arg as *mut Termios;
                unsafe { *ptr = *self.termios.lock(); }
                Ok(0)
            }
            0x5402 => { // TCSETS
                let ptr = arg as *const Termios;
                unsafe { *self.termios.lock() = *ptr; }
                Ok(0)
            }
            0x5413 => { // TIOCGWINSZ
                let ptr = arg as *mut WinSize;
                unsafe { *ptr = *self.winsize.lock(); }
                Ok(0)
            }
            _ => Err("unknown tty ioctl"),
        }
    }

    fn as_any(&self) -> &dyn core::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any { self }
}
