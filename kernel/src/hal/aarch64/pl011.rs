use crate::interfaces::SerialDevice;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::kernel::bit_utils::pl011 as bits;

// Telemetry
pub static SERIAL_TX_BYTES: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_DROPS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_SPIN_LOOPS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_TX_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_RX_BYTES: AtomicU64 = AtomicU64::new(0);
pub static SERIAL_RX_DROPS: AtomicU64 = AtomicU64::new(0);

const SERIAL_TX_MAX_SPINS: usize = 1_000_000;
const SERIAL_INIT_WAIT_MAX_SPINS: usize = 1_000_000;
const UART_CLEAR_ALL_INTERRUPTS: u32 = 0x7FF;
const UART_MIN_CLOCK_DIVIDER_BASE: u32 = 16;

pub const fn tx_timeout_spins() -> usize {
    SERIAL_TX_MAX_SPINS
}

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
        unsafe { read_volatile(self.reg(offset)) }
    }

    #[inline]
    unsafe fn write_reg(&self, offset: usize, val: u32) {
        unsafe { write_volatile(self.reg(offset), val) };
    }

    fn write_byte_with_timeout(&self, data: u8) -> bool {
        let mut spins = 0usize;
        unsafe {
            while bits::FR_TXFF.bit(self.read_reg(bits::UARTFR)) {
                if spins >= SERIAL_TX_MAX_SPINS {
                    SERIAL_TX_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                    SERIAL_TX_DROPS.fetch_add(1, Ordering::Relaxed);
                    SERIAL_TX_SPIN_LOOPS.fetch_add(spins as u64, Ordering::Relaxed);
                    return false;
                }
                spins = spins.saturating_add(1);
                core::hint::spin_loop();
            }
            self.write_reg(bits::UARTDR, data as u32);
        }
        SERIAL_TX_SPIN_LOOPS.fetch_add(spins as u64, Ordering::Relaxed);
        SERIAL_TX_BYTES.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn recv_byte(&self) -> Option<u8> {
        unsafe {
            if bits::FR_RXFE.bit(self.read_reg(bits::UARTFR)) {
                return None;
            }
            let data = self.read_reg(bits::UARTDR) as u8;
            SERIAL_RX_BYTES.fetch_add(1, Ordering::Relaxed);
            Some(data)
        }
    }

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

    pub fn flush(&self) {
        let mut spins = 0usize;
        unsafe {
            while bits::FR_BUSY.bit(self.read_reg(bits::UARTFR))
                || !bits::FR_TXFE_REAL.bit(self.read_reg(bits::UARTFR))
            {
                spins = spins.saturating_add(1);
                if spins > SERIAL_TX_MAX_SPINS {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }

    pub fn clear_interrupts(&self) {
        unsafe {
            self.write_reg(bits::UARTICR, UART_CLEAR_ALL_INTERRUPTS);
        }
    }

    pub fn set_interrupt_mask(&self, rx: bool, tx: bool) {
        unsafe {
            let mut mask = 0u32;
            mask = bits::INT_RX.set_bit(mask, rx);
            mask = bits::INT_TX.set_bit(mask, tx);
            self.write_reg(bits::UARTIMSC, mask);
        }
    }

    pub fn masked_interrupt_status(&self) -> u32 {
        unsafe { self.read_reg(bits::UARTMIS) }
    }

    pub fn is_rx_interrupt_pending(&self) -> bool {
        bits::INT_RX.bit(self.masked_interrupt_status())
    }
}

impl SerialDevice for Pl011Uart {
    fn init(&mut self) {
        if self.baud == 0 || self.clock_hz < UART_MIN_CLOCK_DIVIDER_BASE {
            SERIAL_TX_DROPS.fetch_add(1, Ordering::Relaxed);
            return;
        }

        unsafe {
            self.write_reg(bits::UARTCR, 0);

            let mut spin = 0usize;
            while bits::FR_BUSY.bit(self.read_reg(bits::UARTFR)) && spin < SERIAL_INIT_WAIT_MAX_SPINS {
                spin += 1;
                core::hint::spin_loop();
            }

            self.clear_interrupts();

            let divisor_x16 = self.clock_hz / self.baud;
            let ibrd = divisor_x16 / 16;
            let frac_x16 = self.clock_hz % (16 * self.baud);
            let fbrd = (frac_x16 * 4 + self.baud / 2) / self.baud;

            self.write_reg(bits::UARTIBRD, ibrd);
            self.write_reg(bits::UARTFBRD, fbrd);

            self.write_reg(
                bits::UARTLCR_H,
                bits::LCR_WLEN8.write(0, 0x3) | bits::LCR_FEN.set_bit(0, true),
            );

            let cr = bits::CR_UARTEN.set_bit(0, true)
                | bits::CR_TXE.set_bit(0, true)
                | bits::CR_RXE.set_bit(0, true);
            self.write_reg(bits::UARTCR, cr);
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
