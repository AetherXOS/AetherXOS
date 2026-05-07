use alloc::collections::VecDeque;
use alloc::sync::Arc;

use spin::Mutex;

use crate::modules::vfs::dev_special::{Termios, WinSize};

use super::runtime::pty_runtime_config;
use super::SIGWINCH;

pub(crate) struct PtyPairInner {
    pub(crate) master_to_slave: VecDeque<u8>,
    pub(crate) slave_to_master: VecDeque<u8>,
    pub(crate) index: u32,
    pub(crate) locked: bool,
    pub(crate) termios: Termios,
    pub(crate) winsize: WinSize,
    pub(crate) fg_pgid: i32,
    pub(crate) controlling_session: Option<usize>,
    pub(crate) slave_opened: bool,
    pub(crate) master_closed: bool,
    pub(crate) slave_closed: bool,
}

#[derive(Clone)]
pub(crate) struct PtyPair(pub(crate) Arc<Mutex<PtyPairInner>>);

impl PtyPair {
    pub(crate) fn new(index: u32) -> Self {
        let config = pty_runtime_config();
        Self(Arc::new(Mutex::new(PtyPairInner {
            master_to_slave: VecDeque::with_capacity(super::PTY_BUF_SIZE),
            slave_to_master: VecDeque::with_capacity(super::PTY_BUF_SIZE),
            index,
            locked: config.default_locked,
            termios: Termios::default(),
            winsize: config.default_winsize,
            fg_pgid: 0,
            controlling_session: None,
            slave_opened: false,
            master_closed: false,
            slave_closed: false,
        })))
    }

    pub(crate) fn controlling_session(&self) -> Option<usize> {
        self.0.lock().controlling_session
    }

    pub(crate) fn attach_controlling_session(
        &self,
        session_id: usize,
        foreground_pgid: i32,
    ) -> Result<(), &'static str> {
        if !pty_runtime_config().allow_control_terminal_attach {
            return Err("EPERM");
        }

        let mut inner = self.0.lock();
        match inner.controlling_session {
            Some(existing) if existing != session_id => return Err("EBUSY"),
            _ => {}
        }
        inner.controlling_session = Some(session_id);
        inner.fg_pgid = foreground_pgid;
        Ok(())
    }

    pub(crate) fn detach_controlling_session(&self, session_id: usize) -> Result<(), &'static str> {
        if !pty_runtime_config().allow_control_terminal_detach {
            return Err("EPERM");
        }

        let mut inner = self.0.lock();
        match inner.controlling_session {
            Some(existing) if existing == session_id => {
                inner.controlling_session = None;
                inner.fg_pgid = 0;
                Ok(())
            }
            Some(_) => Err("EPERM"),
            None => Ok(()),
        }
    }

    pub(crate) fn maybe_signal_foreground_group(&self, fg_pgid: i32) {
        if !pty_runtime_config().auto_sigwinch_on_resize {
            return;
        }

        #[cfg(feature = "posix_process")]
        if fg_pgid > 0 {
            let _ = crate::modules::posix::process::killpg(fg_pgid as usize, SIGWINCH);
        }
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.0.lock().locked
    }

    pub(crate) fn set_locked(&self, locked: bool) {
        self.0.lock().locked = locked;
    }

    pub(crate) fn set_slave_opened(&self) {
        self.0.lock().slave_opened = true;
    }

    pub(crate) fn master_closed(&self) -> bool {
        self.0.lock().master_closed
    }

    pub(crate) fn slave_closed(&self) -> bool {
        self.0.lock().slave_closed
    }

    pub(crate) fn mark_master_closed(&self) -> bool {
        let mut inner = self.0.lock();
        inner.master_closed = true;
        inner.master_closed && inner.slave_closed
    }

    pub(crate) fn mark_slave_closed(&self) -> bool {
        let mut inner = self.0.lock();
        inner.slave_closed = true;
        inner.master_closed && inner.slave_closed
    }

    pub(crate) fn get_termios(&self) -> Termios {
        self.0.lock().termios
    }

    pub(crate) fn set_termios(&self, termios: Termios) {
        self.0.lock().termios = termios;
    }

    pub(crate) fn get_winsize(&self) -> WinSize {
        self.0.lock().winsize
    }

    pub(crate) fn set_winsize(&self, winsize: WinSize) -> i32 {
        let mut inner = self.0.lock();
        inner.winsize = winsize;
        inner.fg_pgid
    }

    pub(crate) fn fg_pgid(&self) -> i32 {
        self.0.lock().fg_pgid
    }

    pub(crate) fn set_fg_pgid(&self, fg_pgid: i32) {
        self.0.lock().fg_pgid = fg_pgid;
    }

    pub(crate) fn slave_to_master_len(&self) -> usize {
        self.0.lock().slave_to_master.len()
    }

    pub(crate) fn master_to_slave_len(&self) -> usize {
        self.0.lock().master_to_slave.len()
    }

    pub(crate) fn pop_slave_to_master(&self) -> Option<u8> {
        self.0.lock().slave_to_master.pop_front()
    }

    pub(crate) fn pop_master_to_slave(&self) -> Option<u8> {
        self.0.lock().master_to_slave.pop_front()
    }

    pub(crate) fn push_slave_to_master(&self, byte: u8) -> bool {
        let mut inner = self.0.lock();
        if inner.slave_to_master.len() < super::PTY_BUF_SIZE {
            inner.slave_to_master.push_back(byte);
            true
        } else {
            false
        }
    }

    pub(crate) fn push_master_to_slave(&self, byte: u8) -> bool {
        let mut inner = self.0.lock();
        if inner.master_to_slave.len() < super::PTY_BUF_SIZE {
            inner.master_to_slave.push_back(byte);
            true
        } else {
            false
        }
    }

    pub(crate) fn index(&self) -> u32 {
        self.0.lock().index
    }
}
