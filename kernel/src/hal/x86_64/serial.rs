use crate::interfaces::SerialDevice;
use crate::generated_consts::CORE_CRASH_LOG_CAPACITY;
use crate::kernel::sync::IrqSafeMutex;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;
use crate::kernel::bit_utils::com as bits;

// Standard COM1 port constants moved to bit_utils
const SERIAL_TX_MAX_SPINS: usize = 1_000_000;

static SERIAL_TX_BYTES: AtomicU64 = AtomicU64::new(0);
static SERIAL_TX_DROPS: AtomicU64 = AtomicU64::new(0);
static SERIAL_TX_SPIN_LOOPS: AtomicU64 = AtomicU64::new(0);
static SERIAL_TX_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static TRACE_NEXT_SEQ: AtomicU64 = AtomicU64::new(0);

const TRACE_TEXT_LIMIT: usize = 24;
const TRACE_DUMP_PREVIEW_BYTES: usize = 32;

define_flags!(pub struct TraceFlags: u8 {
    HAS_VALUE = 0x01,
    IS_DUMP   = 0x02,
});

#[derive(Debug, Clone, Copy)]
pub struct TraceRecord {
    pub seq: u64,
    pub flags: TraceFlags,
    pub scope_len: u8,
    pub stage_len: u8,
    pub value: u64,
    pub scope: [u8; TRACE_TEXT_LIMIT],
    pub stage: [u8; TRACE_TEXT_LIMIT],
}

impl TraceRecord {
    pub const EMPTY: Self = Self {
        seq: 0,
        flags: TraceFlags::empty(),
        scope_len: 0,
        stage_len: 0,
        value: 0,
        scope: [0; TRACE_TEXT_LIMIT],
        stage: [0; TRACE_TEXT_LIMIT],
    };

    fn scope_str(&self) -> &str {
        core::str::from_utf8(&self.scope[..self.scope_len as usize]).unwrap_or("?")
    }

    fn stage_str(&self) -> &str {
        core::str::from_utf8(&self.stage[..self.stage_len as usize]).unwrap_or("?")
    }
}

static TRACE_LOG: Mutex<[TraceRecord; CORE_CRASH_LOG_CAPACITY]> =
    Mutex::new([TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY]);

#[derive(Debug, Clone, Copy)]
pub struct SerialRuntimeStats {
    pub tx_bytes: u64,
    pub tx_drops: u64,
    pub tx_spin_loops: u64,
    pub tx_timeouts: u64,
    pub trace_events: u64,
}

pub fn stats() -> SerialRuntimeStats {
    SerialRuntimeStats {
        tx_bytes: SERIAL_TX_BYTES.load(Ordering::Relaxed),
        tx_drops: SERIAL_TX_DROPS.load(Ordering::Relaxed),
        tx_spin_loops: SERIAL_TX_SPIN_LOOPS.load(Ordering::Relaxed),
        tx_timeouts: SERIAL_TX_TIMEOUTS.load(Ordering::Relaxed),
        trace_events: crate::kernel::debug_trace::event_count(),
    }
}

pub struct SerialPort {
    data: Port<u8>,
    ier: Port<u8>,
    fcr: Port<u8>,
    lcr: Port<u8>,
    mcr: Port<u8>,
    lsr: Port<u8>,
}

impl SerialPort {
    pub const fn new(base: u16) -> Self {
        Self {
            data: Port::new(base + bits::DATA),
            ier:  Port::new(base + bits::IER),
            fcr:  Port::new(base + bits::IIR_FCR),
            lcr:  Port::new(base + bits::LCR),
            mcr:  Port::new(base + bits::MCR),
            lsr:  Port::new(base + bits::LSR),
        }
    }

    #[inline(always)]
    fn is_transmit_empty(&mut self) -> bool {
        unsafe { self.lsr.read() & bits::LSR_THRE != 0 }
    }

    fn write_byte_with_timeout(&mut self, data: u8) -> bool {
        let mut spins = 0usize;
        while !self.is_transmit_empty() {
            if spins >= SERIAL_TX_MAX_SPINS {
                SERIAL_TX_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                SERIAL_TX_DROPS.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            spins += 1;
            SERIAL_TX_SPIN_LOOPS.fetch_add(1, Ordering::Relaxed);
            core::hint::spin_loop();
        }
        unsafe { self.data.write(data); }
        SERIAL_TX_BYTES.fetch_add(1, Ordering::Relaxed);
        true
    }
}

impl SerialDevice for SerialPort {
    fn init(&mut self) {
        unsafe {
            self.ier.write(0x00); // Disable interrupts
            self.lcr.write(bits::LCR_DLAB); // Enable DLAB
            self.data.write(0x03); // Divisor 3 (38400 baud)
            self.ier.write(0x00);
            self.lcr.write(bits::LCR_8N1); // 8 bits, no parity, one stop bit
            self.fcr.write(bits::FCR_ENABLE | bits::FCR_CLEAR_RX | bits::FCR_CLEAR_TX | bits::FCR_TRIGGER_LEVEL_14);
            self.mcr.write(bits::MCR_DTR | bits::MCR_RTS | bits::MCR_OUT2);
            self.ier.write(0x01); // Re-enable interrupts
        }
    }

    fn send(&mut self, data: u8) {
        if data == b'\n' { let _ = self.write_byte_with_timeout(b'\r'); }
        let _ = self.write_byte_with_timeout(data);
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() { self.send(byte); }
        Ok(())
    }
}

pub static SERIAL1: IrqSafeMutex<SerialPort> = IrqSafeMutex::new(SerialPort::new(bits::COM1_BASE));

fn copy_trace_text(dst: &mut [u8; TRACE_TEXT_LIMIT], src: &str) -> u8 {
    let bytes = src.as_bytes();
    let len = core::cmp::min(bytes.len(), TRACE_TEXT_LIMIT);
    dst[..len].copy_from_slice(&bytes[..len]);
    len as u8
}

fn record_trace_event(scope: &str, stage: &str, value: Option<u64>, is_dump: bool) {
    let seq = TRACE_NEXT_SEQ.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    let idx = (seq as usize) % CORE_CRASH_LOG_CAPACITY;
    let mut record = TraceRecord::EMPTY;
    record.seq = seq;
    record.scope_len = copy_trace_text(&mut record.scope, scope);
    record.stage_len = copy_trace_text(&mut record.stage, stage);
    if let Some(v) = value {
        record.flags.insert(TraceFlags::HAS_VALUE);
        record.value = v;
    }
    if is_dump { record.flags.insert(TraceFlags::IS_DUMP); }
    TRACE_LOG.lock()[idx] = record;
}

pub fn recent_traces_into(out: &mut [TraceRecord]) -> usize {
    if out.is_empty() { return 0; }
    let total = core::cmp::min(TRACE_NEXT_SEQ.load(Ordering::Relaxed) as usize, CORE_CRASH_LOG_CAPACITY);
    if total == 0 { return 0; }

    let records = TRACE_LOG.lock();
    let n = core::cmp::min(out.len(), total);
    let oldest = if total == CORE_CRASH_LOG_CAPACITY {
        (TRACE_NEXT_SEQ.load(Ordering::Relaxed) as usize) % CORE_CRASH_LOG_CAPACITY
    } else { 0 };
    let start = total.saturating_sub(n);
    let mut cursor = (oldest + start) % CORE_CRASH_LOG_CAPACITY;
    let mut written = 0usize;

    while written < n {
        let record = records[cursor];
        if record.seq != 0 { out[written] = record; written += 1; }
        cursor = (cursor + 1) % CORE_CRASH_LOG_CAPACITY;
    }
    written
}

pub fn write_raw(s: &str) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| { let _ = SERIAL1.lock().write_str(s); });
}

pub const fn tx_timeout_spins() -> usize {
    SERIAL_TX_MAX_SPINS
}

pub fn write_hex(label: &str, value: u64) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    record_trace_event(label, "hex", Some(value), false);
    crate::kernel::debug_trace::record(label, "hex", Some(value), false);
    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = write!(serial, "[EARLY SERIAL] {}={:#x}\n", label, value);
    });
}

pub fn write_trace(scope: &str, stage: &str) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    record_trace_event(scope, stage, None, false);
    crate::kernel::debug_trace::record(scope, stage, None, false);
    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = write!(serial, "[EARLY SERIAL] {} {}\n", scope, stage);
    });
}

pub fn write_trace_hex(scope: &str, key: &str, value: u64) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    record_trace_event(scope, key, Some(value), false);
    crate::kernel::debug_trace::record(scope, key, Some(value), false);
    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = write!(serial, "[EARLY SERIAL] {} {}={:#x}\n", scope, key, value);
    });
}

pub fn write_dump_bytes(label: &str, bytes: &[u8]) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    let preview_len = core::cmp::min(bytes.len(), TRACE_DUMP_PREVIEW_BYTES);
    let mut folded = 0u64;
    for (idx, byte) in bytes.iter().copied().take(8).enumerate() { folded |= (byte as u64) << (idx * 8); }
    record_trace_event(label, "dump", Some(folded), true);
    crate::kernel::debug_trace::record(label, "dump", Some(folded), true);
    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = write!(serial, "[EARLY SERIAL] {} dump len={} data=", label, bytes.len());
        for byte in &bytes[..preview_len] { let _ = write!(serial, "{:02x}", byte); }
        if bytes.len() > preview_len { let _ = write!(serial, "..."); }
        let _ = write!(serial, "\n");
    });
}

pub fn dump_recent_traces() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_traces_into(&mut recent);
    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = write!(serial, "[EARLY SERIAL] trace dump begin count={}\n", written);
        for record in recent.iter().take(written) {
            if record.flags.contains(TraceFlags::HAS_VALUE) {
                let _ = write!(serial, "[EARLY SERIAL] trace seq={} {} {} value={:#x}\n", record.seq, record.scope_str(), record.stage_str(), record.value);
            } else {
                let _ = write!(serial, "[EARLY SERIAL] trace seq={} {} {}\n", record.seq, record.scope_str(), record.stage_str());
            }
        }
        let _ = write!(serial, "[EARLY SERIAL] trace dump end\n");
    });
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| { let _ = SERIAL1.lock().write_fmt(args); });
}

pub fn init() { SERIAL1.lock().init(); }

#[cfg(test)]
mod tests {
    use super::*;
    #[test_case]
    fn trace_ring_records_recent_events_in_order() {
        record_trace_event("trace.test", "alpha", None, false);
        record_trace_event("trace.test", "beta", Some(0x44), false);
        let mut recent = [TraceRecord::EMPTY; 4];
        let written = recent_traces_into(&mut recent);
        assert!(written >= 2);
        let last = recent[written - 1];
        let prev = recent[written - 2];
        assert_eq!(prev.scope_str(), "trace.test");
        assert_eq!(prev.stage_str(), "alpha");
        assert_eq!(last.scope_str(), "trace.test");
        assert_eq!(last.stage_str(), "beta");
        assert_eq!(last.value, 0x44);
    }
}
