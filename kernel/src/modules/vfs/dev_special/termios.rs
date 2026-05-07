/// Terminal line discipline flags.
#[derive(Debug, Clone, Copy)]
pub struct Termios {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub cc: [u8; 32],
}

impl Default for Termios {
    fn default() -> Self {
        let mut cc = [0u8; 32];
        cc[0] = 0x03;
        cc[1] = 0x1C;
        cc[2] = 0x7F;
        cc[3] = 0x15;
        cc[4] = 0x04;
        cc[5] = 0x00;
        cc[6] = 0x01;
        cc[7] = 0x00;
        cc[8] = 0x11;
        cc[9] = 0x13;
        cc[10] = 0x1A;
        cc[11] = 0x00;
        cc[12] = 0x12;
        cc[13] = 0x0F;
        cc[14] = 0x17;
        cc[15] = 0x16;
        cc[16] = 0x00;

        Self {
            iflag: 0o013602,
            oflag: 0o000005,
            cflag: 0o000277,
            lflag: 0o105073,
            cc,
        }
    }
}

pub use crate::kernel::tty::WinSize;

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
