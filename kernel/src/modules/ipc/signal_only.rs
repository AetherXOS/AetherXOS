use crate::interfaces::IpcChannel;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};
use spin::Mutex;

declare_counter_u64!(SIGNAL_SEND_CALLS);
declare_counter_u64!(SIGNAL_RECEIVE_CALLS);
declare_counter_u64!(SIGNAL_RECEIVE_HITS);

#[derive(Debug, Clone, Copy)]
pub struct SignalOnlyStats {
    pub send_calls: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
}

pub fn stats() -> SignalOnlyStats {
    SignalOnlyStats {
        send_calls: telemetry::snapshot_u64(&SIGNAL_SEND_CALLS),
        receive_calls: telemetry::snapshot_u64(&SIGNAL_RECEIVE_CALLS),
        receive_hits: telemetry::snapshot_u64(&SIGNAL_RECEIVE_HITS),
    }
}

pub fn take_stats() -> SignalOnlyStats {
    SignalOnlyStats {
        send_calls: telemetry::take_u64(&SIGNAL_SEND_CALLS),
        receive_calls: telemetry::take_u64(&SIGNAL_RECEIVE_CALLS),
        receive_hits: telemetry::take_u64(&SIGNAL_RECEIVE_HITS),
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
        counter_inc!(SIGNAL_SEND_CALLS);
        // Ignore payload, just set a bit
        let mut lock = self.signals.lock();
        *lock |= 1;
    }

    fn receive(&self, _buffer: &mut [u8]) -> Option<usize> {
        counter_inc!(SIGNAL_RECEIVE_CALLS);
        let mut lock = self.signals.lock();
        if *lock & 1 != 0 {
            *lock &= !1; // Clear bit
            counter_inc!(SIGNAL_RECEIVE_HITS);
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
