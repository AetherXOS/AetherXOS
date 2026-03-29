use super::pl011::Pl011Uart;
use crate::kernel::sync::IrqSafeMutex;
use core::fmt::Write;

// Default UART base on QEMU virt
pub static SERIAL1: IrqSafeMutex<Pl011Uart> = IrqSafeMutex::new(Pl011Uart::new(0x0900_0000));

#[derive(Debug, Clone, Copy, Default)]
pub struct SerialRuntimeStats {
    pub tx_bytes: u64,
    pub tx_drops: u64,
    pub tx_spin_loops: u64,
    pub tx_timeouts: u64,
    pub trace_events: u64,
}

pub fn stats() -> SerialRuntimeStats {
    SerialRuntimeStats {
        tx_bytes: super::pl011::SERIAL_TX_BYTES.load(core::sync::atomic::Ordering::Relaxed),
        tx_drops: super::pl011::SERIAL_TX_DROPS.load(core::sync::atomic::Ordering::Relaxed),
        tx_spin_loops: super::pl011::SERIAL_TX_SPIN_LOOPS
            .load(core::sync::atomic::Ordering::Relaxed),
        tx_timeouts: super::pl011::SERIAL_TX_TIMEOUTS.load(core::sync::atomic::Ordering::Relaxed),
        trace_events: crate::kernel::debug_trace::event_count(),
    }
}

pub const fn tx_timeout_spins() -> usize {
    super::pl011::tx_timeout_spins()
}

pub fn write_raw(s: &str) {
    let mut serial = SERIAL1.lock();
    let _ = serial.write_str(s);
}

pub fn write_hex(label: &str, value: u64) {
    crate::kernel::debug_trace::record(label, "hex", Some(value), false);
    let mut serial = SERIAL1.lock();
    let _ = write!(serial, "[EARLY SERIAL] {}={:#x}\n", label, value);
}

pub fn write_trace(scope: &str, stage: &str) {
    crate::kernel::debug_trace::record(scope, stage, None, false);
    let mut serial = SERIAL1.lock();
    let _ = write!(serial, "[EARLY SERIAL] {} {}\n", scope, stage);
}

pub fn write_trace_hex(scope: &str, key: &str, value: u64) {
    crate::kernel::debug_trace::record(scope, key, Some(value), false);
    let mut serial = SERIAL1.lock();
    let _ = write!(serial, "[EARLY SERIAL] {} {}={:#x}\n", scope, key, value);
}

pub fn write_dump_bytes(label: &str, bytes: &[u8]) {
    let preview_len = core::cmp::min(bytes.len(), 32);
    let mut folded = 0u64;
    for (idx, byte) in bytes.iter().copied().take(8).enumerate() {
        folded |= (byte as u64) << (idx * 8);
    }
    crate::kernel::debug_trace::record(label, "dump", Some(folded), true);
    let mut serial = SERIAL1.lock();
    let _ = write!(serial, "[EARLY SERIAL] {} dump len={} data=", label, bytes.len());
    for byte in &bytes[..preview_len] {
        let _ = write!(serial, "{:02x}", byte);
    }
    if bytes.len() > preview_len {
        let _ = write!(serial, "...");
    }
    let _ = write!(serial, "\n");
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let mut serial = SERIAL1.lock();
    let _ = serial.write_fmt(args);
}
