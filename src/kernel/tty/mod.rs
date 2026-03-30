//! TTY Device Model & Job Control Integration
//!
//! This module provides a Linux-compatible TTY subsystem, including:
//! - Virtual terminal abstraction (tty_struct equivalent)
//! - Job control via SIGTSTP/SIGCONT
//! - Process group/session lifecycle
//! - Terminal I/O control (tcsetattr, tcgetattr)
//! - Signal delivery to process groups
//!
//! # Architecture
//!
//! The TTY system is organized into:
//! 1. `TtyDevice` - Core TTY abstraction (represents one virtual terminal)
//! 2. Job control (`job_control.rs`) - Process group/session + signal delivery
//! 3. Terminal I/O control - tcsetattr/tcgetattr attributes
//! 4. Discipline integration - Line discipline callbacks
//!
//! # State Machine
//!
//! ```text
//! [Uninitialized] → [Initialized] → [Active Session] ↔ [Stopped/Suspended]
//!                        ↓                                     ↓
//!                    [Orphaned]                           [Background]
//! ```

mod job_control;

pub use job_control::{JobControlState, ProcessGroupId, SessionId};

use crate::interfaces::task::ProcessId;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// TTY Device identifier (typically 0-255 for /dev/tty0, /dev/tty1, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TtyId(pub u32);

impl TtyId {
    pub fn new(id: u32) -> Self {
        TtyId(id)
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Terminal I/O control attributes (mirrors Linux termios)
#[derive(Debug, Clone)]
pub struct TermiosAttrs {
    // Input flags (c_iflag)
    pub ignore_break: bool,
    pub signal_interrupt_on_break: bool,
    pub ignore_parity_errors: bool,
    pub mark_parity_errors: bool,
    pub check_parity: bool,
    pub convert_cr_to_lf: bool,
    pub ignore_cr: bool,

    // Output flags (c_oflag)
    pub post_process_output: bool,
    pub expand_tab_to_spaces: bool,

    // Control flags (c_cflag)
    pub char_size_bits: u8, // 5=CS5, 6=CS6, 7=CS7, 8=CS8
    pub two_stop_bits: bool,
    pub enable_receiver: bool,

    // Local flags (c_lflag)
    pub canonical_input: bool,
    pub echo_input: bool,
    pub echo_erase_char: bool,
    pub echo_kill_char: bool,
    pub enable_signals: bool, // Enable ISIG processing (SIGINT, SIGQUIT)
    pub extended_input_processing: bool,

    // Special characters (c_cc array)
    pub veof: u8,  // EOF character (typically Ctrl-D)
    pub veol: u8,  // EOL character
    pub verase: u8, // ERASE character (typically Ctrl-H or Backspace)
    pub vkill: u8,  // KILL character (typically Ctrl-U)
    pub vintr: u8,  // INTR character (typically Ctrl-C)
    pub vquit: u8,  // QUIT character (typically Ctrl-\)
    pub vsusp: u8,  // SUSP character (typically Ctrl-Z)
}

impl Default for TermiosAttrs {
    fn default() -> Self {
        TermiosAttrs {
            ignore_break: false,
            signal_interrupt_on_break: false,
            ignore_parity_errors: false,
            mark_parity_errors: false,
            check_parity: false,
            convert_cr_to_lf: false,
            ignore_cr: false,

            post_process_output: false,
            expand_tab_to_spaces: false,

            char_size_bits: 8,
            two_stop_bits: false,
            enable_receiver: true,

            canonical_input: true,
            echo_input: true,
            echo_erase_char: true,
            echo_kill_char: true,
            enable_signals: true,
            extended_input_processing: true,

            veof: 4,     // Ctrl-D
            veol: 0,     // None
            verase: 127, // Backspace
            vkill: 21,   // Ctrl-U
            vintr: 3,    // Ctrl-C
            vquit: 28,   // Ctrl-\
            vsusp: 26,   // Ctrl-Z
        }
    }
}

/// TTY Device (equivalent to Linux `struct tty_struct`)
///
/// Represents a single virtual terminal instance. Multiple processes can
/// share a single TTY via job control (foreground/background groups).
#[derive(Debug)]
pub struct TtyDevice {
    /// TTY device identifier
    id: TtyId,

    /// Terminal I/O attributes
    termios: TermiosAttrs,

    /// Current session ID holding this TTY
    session_id: core::sync::atomic::AtomicUsize,

    /// Current foreground process group ID (None if no foreground group)
    foreground_pgrp: core::sync::atomic::AtomicUsize,

    /// Whether the TTY is open/active
    is_open: AtomicBool,

    /// Input queue (for line buffering in canonical mode)
    #[allow(dead_code)]
    input_buffer: core::sync::atomic::AtomicPtr<u8>,

    /// Output queue position (bytes written)
    #[allow(dead_code)]
    output_pos: AtomicU32,

    /// Job control state (process groups, sessions, suspension state)
    #[allow(dead_code)]
    job_control: core::sync::atomic::AtomicPtr<JobControlState>,
}

impl TtyDevice {
    /// Create a new TTY device with the given ID
    pub fn new(id: TtyId) -> Self {
        TtyDevice {
            id,
            termios: TermiosAttrs::default(),
            session_id: core::sync::atomic::AtomicUsize::new(usize::MAX),
            foreground_pgrp: core::sync::atomic::AtomicUsize::new(usize::MAX),
            is_open: AtomicBool::new(false),
            input_buffer: core::sync::atomic::AtomicPtr::new(core::ptr::null_mut()),
            output_pos: AtomicU32::new(0),
            job_control: core::sync::atomic::AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Get the TTY device ID
    pub fn id(&self) -> TtyId {
        self.id
    }

    /// Open the TTY device
    pub fn open(&self) -> crate::interfaces::KernelResult<()> {
        self.is_open.store(true, Ordering::Release);
        Ok(())
    }

    /// Close the TTY device
    pub fn close(&self) {
        self.is_open.store(false, Ordering::Release);
    }

    /// Check if the TTY is open
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Acquire)
    }

    /// Get the current terminal attributes
    pub fn get_termios(&self) -> TermiosAttrs {
        self.termios.clone()
    }

    /// Set new terminal attributes
    pub fn set_termios(&mut self, attrs: TermiosAttrs) {
        self.termios = attrs;
    }

    /// Get the foreground process group ID
    pub fn foreground_pgrp(&self) -> Option<ProcessGroupId> {
        let pgrp = self.foreground_pgrp.load(Ordering::Acquire);
        if pgrp == usize::MAX {
            None
        } else {
            Some(ProcessGroupId(ProcessId(pgrp)))
        }
    }

    /// Set the foreground process group ID
    pub fn set_foreground_pgrp(&self, pgrp: Option<ProcessGroupId>) {
        let val = pgrp.map(|p| (p.0).0).unwrap_or(usize::MAX);
        self.foreground_pgrp.store(val, Ordering::Release);
    }

    /// Get the session ID
    pub fn session_id(&self) -> Option<SessionId> {
        let sid = self.session_id.load(Ordering::Acquire);
        if sid == usize::MAX {
            None
        } else {
            Some(SessionId(ProcessId(sid)))
        }
    }

    /// Set the session ID
    pub fn set_session_id(&self, sid: Option<SessionId>) {
        let val = sid.map(|s| (s.0).0).unwrap_or(usize::MAX);
        self.session_id.store(val, Ordering::Release);
    }

    /// Write raw data to TTY (called by kernel output paths)
    pub fn write(&self, _data: &[u8]) -> crate::interfaces::KernelResult<usize> {
        // TODO: Implement actual output queuing and discipline processing
        Ok(0)
    }

    /// Read data from TTY (blocking, handles line buffering in canonical mode)
    pub fn read(&self, _buf: &mut [u8]) -> crate::interfaces::KernelResult<usize> {
        // TODO: Implement input line buffering + echo
        Ok(0)
    }
}

/// Global TTY registry (up to 256 virtual terminals)
pub struct TtyRegistry {
    devices: [Option<Arc<TtyDevice>>; 256],
}

impl TtyRegistry {
    pub fn new() -> Self {
        const INIT: Option<Arc<TtyDevice>> = None;
        TtyRegistry {
            devices: [INIT; 256],
        }
    }

    /// Register a TTY device
    pub fn register(&mut self, id: TtyId, device: Arc<TtyDevice>) -> crate::interfaces::KernelResult<()> {
        if id.0 >= 256 {
            return Err(crate::interfaces::KernelError::InvalidInput);
        }
        self.devices[id.0 as usize] = Some(device);
        Ok(())
    }

    /// Get a TTY device by ID
    pub fn get(&self, id: TtyId) -> Option<Arc<TtyDevice>> {
        if id.0 >= 256 {
            return None;
        }
        self.devices[id.0 as usize].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn tty_device_creation() {
        let tty = TtyDevice::new(TtyId::new(0));
        assert_eq!(tty.id(), TtyId::new(0));
        assert!(!tty.is_open());
    }

    #[test_case]
    fn tty_device_open_close() {
        let tty = TtyDevice::new(TtyId::new(0));
        tty.open().unwrap();
        assert!(tty.is_open());
        tty.close();
        assert!(!tty.is_open());
    }

    #[test_case]
    fn tty_foreground_pgrp() {
        let tty = TtyDevice::new(TtyId::new(0));
        assert_eq!(tty.foreground_pgrp(), None);

        let pgrp = ProcessGroupId(crate::interfaces::task::ProcessId(1000));
        tty.set_foreground_pgrp(Some(pgrp));
        assert_eq!(tty.foreground_pgrp(), Some(pgrp));

        tty.set_foreground_pgrp(None);
        assert_eq!(tty.foreground_pgrp(), None);
    }

    #[test_case]
    fn tty_session_id() {
        let tty = TtyDevice::new(TtyId::new(0));
        assert_eq!(tty.session_id(), None);

        let sid = SessionId(crate::interfaces::task::ProcessId(2000));
        tty.set_session_id(Some(sid));
        assert_eq!(tty.session_id(), Some(sid));

        tty.set_session_id(None);
        assert_eq!(tty.session_id(), None);
    }

    #[test_case]
    fn termios_default_values() {
        let attrs = TermiosAttrs::default();
        assert_eq!(attrs.char_size_bits, 8);
        assert!(attrs.canonical_input);
        assert!(attrs.echo_input);
        assert!(attrs.enable_signals);
        assert_eq!(attrs.vintr, 3); // Ctrl-C
        assert_eq!(attrs.vsusp, 26); // Ctrl-Z
    }

    #[test_case]
    fn tty_registry_basic() {
        let mut registry = TtyRegistry::new();
        let tty = Arc::new(TtyDevice::new(TtyId::new(0)));
        registry.register(TtyId::new(0), tty.clone()).unwrap();

        let retrieved = registry.get(TtyId::new(0));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), TtyId::new(0));
    }

    #[test_case]
    fn tty_registry_out_of_bounds() {
        let registry = TtyRegistry::new();
        let retrieved = registry.get(TtyId::new(256));
        assert!(retrieved.is_none());
    }
}
