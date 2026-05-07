//! Special device file implementations for Linux compatibility.
//!
//! Provides /dev/null, /dev/zero, /dev/full, /dev/random, /dev/urandom,
//! /dev/tty, /dev/console, /dev/stdin, /dev/stdout, /dev/stderr.

use core::any::Any;
use crate::modules::vfs::{File, FileStats, PollEvents};

mod prng;
mod simple;
mod termios;
mod tty;
#[path = "stdio.rs"]
mod stdio;

pub use prng::seed_prng;
pub use simple::{DevNull, DevZero, DevFull, DevRandom};
pub use termios::{Termios, WinSize, TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGPGRP, TIOCSPGRP, TIOCGWINSZ, TIOCSWINSZ, TIOCSCTTY, TIOCNOTTY, FIONREAD, FIONBIO};
pub use tty::DevTty;
pub use stdio::register_linux_special_devices;
