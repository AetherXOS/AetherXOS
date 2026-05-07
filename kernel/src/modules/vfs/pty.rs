//! PTY (Pseudo-Terminal) subsystem for Linux compatibility.
//!
//! Implements /dev/ptmx (master allocator) and /dev/pts/* (slave terminals).
//! This is essential for SSH, tmux, screen, and interactive shells.

extern crate alloc;

use alloc::boxed::Box;

use crate::modules::vfs::types::{DirEntry, File, FileStats};

pub use crate::modules::vfs::dev_special::{
    FIONBIO, FIONREAD, TCGETS, TCSETS, TCSETSF, TCSETSW, TIOCGPGRP, TIOCGWINSZ, TIOCNOTTY,
    TIOCSPGRP, TIOCSCTTY, TIOCSWINSZ, Termios, WinSize,
};

const PTY_BUF_SIZE: usize = 4096;

pub const TIOCSPTLCK: u32 = 0x40045431;
pub const TIOCGPTN: u32 = 0x80045430;
pub(crate) const SIGWINCH: i32 = 28;

mod runtime;
mod pair;
mod ioctl;
mod master;
mod slave;
mod registry;
mod dev;
mod fs;
mod tests;

pub use dev::{open_ptmx, open_pts, DevPtmx};
pub use fs::PtsFs;
pub use registry::init_pty_subsystem;
pub use runtime::{configure_pty_runtime, pty_runtime_config, PtyRuntimeConfig};
#[cfg(test)]
pub(super) use runtime::reset_pty_runtime_config;

pub(crate) use master::PtyMaster;
pub(crate) use pair::PtyPair;
pub(crate) use slave::PtySlave;
