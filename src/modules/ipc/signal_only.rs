use crate::interfaces::IpcChannel;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

static SIGNAL_SEND_CALLS: AtomicU64 = AtomicU64::new(0);
static SIGNAL_RECEIVE_CALLS: AtomicU64 = AtomicU64::new(0);
static SIGNAL_RECEIVE_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct SignalOnlyStats {
    pub send_calls: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
}

pub fn stats() -> SignalOnlyStats {
    SignalOnlyStats {
        send_calls: SIGNAL_SEND_CALLS.load(Ordering::Relaxed),
        receive_calls: SIGNAL_RECEIVE_CALLS.load(Ordering::Relaxed),
        receive_hits: SIGNAL_RECEIVE_HITS.load(Ordering::Relaxed),
    }
}

/// Signal Only IPC.
/// Extremely lightweight "Kick". Used for notifying threads to wake up.
/// No data payload is transferred.

pub struct SignalOnly {
    signals: Mutex<u64>, // Bitmap of signals
}

impl SignalOnly {
    pub const fn new() -> Self {
        Self {
            signals: Mutex::new(0),
        }
    }
}

impl IpcChannel for SignalOnly {
    fn send(&self, _msg: &[u8]) {
        SIGNAL_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        // Ignore payload, just set a bit
        let mut lock = self.signals.lock();
        *lock |= 1;
    }

    fn receive(&self, _buffer: &mut [u8]) -> Option<usize> {
        SIGNAL_RECEIVE_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut lock = self.signals.lock();
        if *lock & 1 != 0 {
            *lock &= !1; // Clear bit
            SIGNAL_RECEIVE_HITS.fetch_add(1, Ordering::Relaxed);
            Some(0)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn signal_only_roundtrip_sets_and_clears_signal() {
        let sig = SignalOnly::new();
        let mut out = [0u8; 1];
        assert_eq!(sig.receive(&mut out), None);
        sig.send(&[]);
        assert_eq!(sig.receive(&mut out), Some(0));
        assert_eq!(sig.receive(&mut out), None);
    }
}
