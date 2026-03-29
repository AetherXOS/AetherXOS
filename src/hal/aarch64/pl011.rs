use crate::interfaces::SerialDevice;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU64, Ordering};

// ── PL011 UART register offsets ───────────────────────────────────────────────
const UARTDR: usize = 0x000; // Data register (TX+RX)
const UARTFR: usize = 0x018; // Flag register
const UARTIBRD: usize = 0x024; // Integer baud rate divisor
const UARTFBRD: usize = 0x028; // Fractional baud rate divisor
const UARTLCR_H: usize = 0x02C; // Line control
const UARTCR: usize = 0x030; // Control register
const UARTIMSC: usize = 0x038; // Interrupt mask
const UARTICR: usize = 0x044; // Interrupt clear

// Flag register bits
const FR_TXFF: u32 = 1 << 5; // TX FIFO full
const FR_RXFE: u32 = 1 << 4; // RX FIFO empty
const FR_BUSY: u32 = 1 << 3; // UART busy
const FR_TXFE: u32 = 1 << 7; // TX FIFO empty

// Control register bits
const CR_UARTEN: u32 = 1 << 0; // UART enable
const CR_TXE: u32 = 1 << 8; // TX enable
const CR_RXE: u32 = 1 << 9; // RX enable

// Line control bits
const LCR_FEN: u32 = 1 << 4; // FIFO enable
const LCR_WLEN8: u32 = 0b11 << 5; // 8-bit word length

// ── Telemetry ──────────────────────────────────────────────────────────────────
pub static SERIAL_TX_BYTES: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_DROPS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_SPIN_LOOPS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_RX_BYTES: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_RX_DROPS: AtomicU64 = AtomicU64::new(0);

const SERIAL_TX_MAX_SPINS: usize = 1_000_000;

pub const fn tx_timeout_spins() -> usize {
    SERIAL_TX_MAX_SPINS
}

// ── PL011 Uart ─────────────────────────────────────────────────────────────────

pub struct Pl011Uart {
    base_addr: usize,
    clock_hz: u32,
    baud: u32,
}

impl Pl011Uart {
    pub const fn new(base_addr: usize) -> Self {
        Self {
            base_addr,
            clock_hz: 48_000_000,
            baud: 115_200,
        }
    }

    pub const fn with_clock(base_addr: usize, clock_hz: u32, baud: u32) -> Self {
        Self {
            base_addr,
            clock_hz,
            baud,
        }
    }

    #[inline]
    fn reg(&self, offset: usize) -> *mut u32 {
        (self.base_addr + offset) as *mut u32
    }

    #[inline]
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        read_volatile(self.reg(offset))
    }

    #[inline]
    unsafe fn write_reg(&self, offset: usize, val: u32) {
        write_volatile(self.reg(offset), val);
    }

    /// Wait for the TX FIFO to have space, or give up on timeout.
    fn write_byte_with_timeout(&self, data: u8) -> bool {
        let mut spins = 0usize;
        unsafe {
            while (self.read_reg(UARTFR) & FR_TXFF) != 0 {
                if spins >= SERIAL_TX_MAX_SPINS {
                    SERIAL_TX_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                    SERIAL_TX_DROPS.fetch_add(1, Ordering::Relaxed);
                    SERIAL_TX_SPIN_LOOPS.fetch_add(spins as u64, Ordering::Relaxed);
                    return false;
                }
                spins = spins.saturating_add(1);
                core::hint::spin_loop();
            }
            self.write_reg(UARTDR, data as u32);
        }
        SERIAL_TX_SPIN_LOOPS.fetch_add(spins as u64, Ordering::Relaxed);
        SERIAL_TX_BYTES.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Non-blocking RX byte read.  Returns `None` if RX FIFO is empty.
    pub fn recv_byte(&self) -> Option<u8> {
        unsafe {
            if (self.read_reg(UARTFR) & FR_RXFE) != 0 {
                return None;
            }
            let data = self.read_reg(UARTDR) as u8;
            SERIAL_RX_BYTES.fetch_add(1, Ordering::Relaxed);
            Some(data)
        }
    }

    /// Drain the RX FIFO into `buf`.  Returns the number of bytes read.
    pub fn recv_burst(&self, buf: &mut [u8]) -> usize {
        let mut n = 0;
        while n < buf.len() {
            match self.recv_byte() {
                Some(b) => {
                    buf[n] = b;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }

    /// Flush TX: spin until TX FIFO is empty and UART not busy.
    pub fn flush(&self) {
        let mut spins = 0usize;
        unsafe {
            while (self.read_reg(UARTFR) & (FR_BUSY | FR_TXFE)) != FR_TXFE {
                spins = spins.saturating_add(1);
                if spins > SERIAL_TX_MAX_SPINS {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }

    /// Clear all pending interrupts.
    pub fn clear_interrupts(&self) {
        unsafe {
            self.write_reg(UARTICR, 0x7FF);
        }
    }

    /// Enable or disable the RX interrupt (UART_IMSC bit 4 = RXIM).
    pub fn set_rx_interrupt(&self, enable: bool) {
        unsafe {
            let mut mask = self.read_reg(UARTIMSC);
            if enable {
                mask |= 1 << 4;
            } else {
                mask &= !(1 << 4);
            }
            self.write_reg(UARTIMSC, mask);
        }
    }
}

impl SerialDevice for Pl011Uart {
    fn init(&mut self) {
        if self.baud == 0 || self.clock_hz < 16 {
            SERIAL_TX_DROPS.fetch_add(1, Ordering::Relaxed);
            return;
        }
        unsafe {
            // 1. Disable UART while reconfiguring.
            self.write_reg(UARTCR, 0);

            // 2. Wait for any in-progress transmission to finish.
            let mut spin = 0usize;
            while (self.read_reg(UARTFR) & FR_BUSY) != 0 && spin < 1_000_000 {
                spin += 1;
                core::hint::spin_loop();
            }

            // 3. Clear all interrupts.
            self.clear_interrupts();

            // 4. Compute baud rate divisors.
            //    Baud rate divisor = UART clock / (16 × baud rate)
            //    Integer part:    IBRD = floor(divisor)
            //    Fractional part: FBRD = round((fractional × 64) + 0.5)
            let divisor_x16 = self.clock_hz / self.baud;
            let ibrd = divisor_x16 / 16;
            let frac_x16 = self.clock_hz % (16 * self.baud);
            let fbrd = (frac_x16 * 4 + self.baud / 2) / self.baud;

            self.write_reg(UARTIBRD, ibrd);
            self.write_reg(UARTFBRD, fbrd);

            // 5. 8-bit word length, no parity, 1 stop bit, FIFO enabled.
            self.write_reg(UARTLCR_H, LCR_WLEN8 | LCR_FEN);

            // 6. Enable UART, TX, RX.
            self.write_reg(UARTCR, CR_UARTEN | CR_TXE | CR_RXE);
        }
    }

    fn send(&mut self, data: u8) {
        if data == b'\n' {
            let _ = self.write_byte_with_timeout(b'\r');
        }
        let _ = self.write_byte_with_timeout(data);
    }
}

impl core::fmt::Write for Pl011Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}
