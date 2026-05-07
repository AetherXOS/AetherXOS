use alloc::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signal {
    pub nr: u32,
    pub source_pid: u32,
    pub timestamp_ns: u64,
}

#[derive(Debug)]
pub struct SignalQueue {
    queue: VecDeque<Signal>,
    pending_mask: u64,
}

impl SignalQueue {
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            pending_mask: 0,
        }
    }

    /// Push a signal into the queue.
    pub fn push(&mut self, sig: Signal) {
        if self.queue.len() < 256 { // Kernel-enforced limit for safety
            self.queue.push_back(sig);
            self.pending_mask |= 1 << (sig.nr - 1);
        }
    }

    /// Pop the next pending signal.
    pub fn pop(&mut self) -> Option<Signal> {
        let sig = self.queue.pop_front();
        if let Some(s) = sig {
            // Recalculate mask if needed, or just clear if empty
            if self.queue.is_empty() {
                self.pending_mask = 0;
            }
            return Some(s);
        }
        None
    }

    pub fn has_pending(&self) -> bool {
        self.pending_mask != 0
    }
}
