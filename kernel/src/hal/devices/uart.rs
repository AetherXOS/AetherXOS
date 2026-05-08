use core::fmt::{self, Write};
use crate::core::types::MmioBase;
use crate::hal::mmio::typed_mmio::MappedRegion;

/// Minimal UART device mapped at a const generic base address.
pub struct Uart<const BASE: usize> {
    _base: MmioBase<BASE>,
}

impl<const BASE: usize> Uart<BASE> {
    pub const fn new() -> Self {
        Self { _base: MmioBase::<BASE> }
    }

    /// Initialize UART. Hardware-specific; minimal stub here.
    pub fn init(&self) {
        // For a real UART driver we'd set baud/ctrl registers here.
        // SAFETY: Writes to UART registers require that the MMIO range is valid.
        unsafe {
            // Example: write 0 to control register at offset 0x00
            let _ : () = MappedRegion::<BASE>::write_offset::<u32>(0x00, 0u32);
        }
    }

    /// Send single byte.
    pub fn send_byte(&self, b: u8) {
        unsafe {
            MappedRegion::<BASE>::write_offset::<u8>(0x00, b);
        }
    }
}

impl<const BASE: usize> Write for Uart<BASE> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            self.send_byte(b);
        }
        Ok(())
    }
}
