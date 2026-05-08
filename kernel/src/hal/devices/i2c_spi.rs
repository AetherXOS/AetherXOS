/// SPI (Serial Peripheral Interface) device abstraction.
///
/// Provides a type-safe interface for SPI bus communication with
/// const-generic base addresses and state tracking.

use super::generic::{GenericMmioDevice, DeviceState, MmioDevice};

/// SPI bus mode (CPOL and CPHA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMode {
    /// Mode 0: CPOL=0, CPHA=0
    Mode0 = 0,
    /// Mode 1: CPOL=0, CPHA=1
    Mode1 = 1,
    /// Mode 2: CPOL=1, CPHA=0
    Mode2 = 2,
    /// Mode 3: CPOL=1, CPHA=1
    Mode3 = 3,
}

/// SPI clock speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiSpeed {
    /// 1 MHz
    Slow1MHz = 1_000_000,
    /// 10 MHz
    Medium10MHz = 10_000_000,
    /// 25 MHz
    Fast25MHz = 25_000_000,
    /// 50 MHz
    VeryFast50MHz = 50_000_000,
}

/// SPI chip select polarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsPolarity {
    /// Chip select active low (typical).
    ActiveLow,
    /// Chip select active high.
    ActiveHigh,
}

/// SPI register offsets (common across most SPI controllers).
#[repr(usize)]
pub enum SpiRegister {
    /// Control Register 0
    Control0 = 0x00,
    /// Control Register 1
    Control1 = 0x04,
    /// Data Register (read and write)
    Data = 0x08,
    /// Status Register
    Status = 0x0C,
    /// Interrupt Status Register
    IntStatus = 0x10,
    /// Interrupt Enable Register
    IntEnable = 0x14,
}

/// Generic SPI device with const-generic MMIO base address.
///
/// # Type Parameters
/// - `BASE`: The memory-mapped base address of the SPI controller.
pub struct SpiDevice<const BASE: usize> {
    generic: GenericMmioDevice<BASE, SpiDeviceTag>,
    mode: SpiMode,
    speed: SpiSpeed,
    cs_polarity: CsPolarity,
}

/// Type tag for SPI devices (used for type-level distinction).
pub struct SpiDeviceTag;

impl<const BASE: usize> SpiDevice<BASE> {
    /// Create a new SPI device in uninitialized state.
    ///
    /// # Arguments
    /// - `mode`: The SPI bus mode (0-3).
    /// - `speed`: The SPI clock speed.
    /// - `cs_polarity`: Chip select polarity.
    pub const fn new(mode: SpiMode, speed: SpiSpeed, cs_polarity: CsPolarity) -> Self {
        SpiDevice {
            generic: GenericMmioDevice::new(),
            mode,
            speed,
            cs_polarity,
        }
    }

    /// Initialize the SPI bus with the configured settings.
    ///
    /// # Safety
    /// BASE must point to a valid SPI controller.
    pub unsafe fn init(&mut self) -> Result<(), &'static str> { unsafe {
        self.generic.init()?;

        // Configure control register 0 with mode and speed
        let cpol = ((self.mode as u32) >> 1) & 1;
        let cpha = (self.mode as u32) & 1;
        let dss = 7; // 8-bit data size
        
        let ctrl0 = dss | (cpol << 6) | (cpha << 7);
        self.generic.write_reg(SpiRegister::Control0 as usize, ctrl0);

        // Configure clock divider based on speed
        let divider = match self.speed {
            SpiSpeed::Slow1MHz => 50,
            SpiSpeed::Medium10MHz => 5,
            SpiSpeed::Fast25MHz => 2,
            SpiSpeed::VeryFast50MHz => 1,
        };
        
        self.generic.write_reg(SpiRegister::Control1 as usize, divider);

        // Enable SPI (set bit 1 in control0)
        self.generic.modify_reg(SpiRegister::Control0 as usize, |val| val | (1 << 0));
        
        self.generic.set_state(DeviceState::Ready);
        Ok(())
    }}

    /// Select a slave device on the SPI bus.
    ///
    /// # Safety
    /// Device must be initialized.
    pub unsafe fn select_slave(&mut self) -> Result<(), &'static str> { unsafe {
        if !self.generic.is_ready() {
            return Err("SPI device not ready");
        }

        // Set chip select active (implementation depends on polarity)
        match self.cs_polarity {
            CsPolarity::ActiveLow => {
                self.generic.modify_reg(SpiRegister::Control1 as usize, |val| val & !(1 << 3));
            }
            CsPolarity::ActiveHigh => {
                self.generic.modify_reg(SpiRegister::Control1 as usize, |val| val | (1 << 3));
            }
        }

        self.generic.set_state(DeviceState::Enabled);
        Ok(())
    }}

    /// Deselect all slave devices on the SPI bus.
    ///
    /// # Safety
    /// A slave must be selected.
    pub unsafe fn deselect_slave(&mut self) -> Result<(), &'static str> { unsafe {
        if !self.generic.is_enabled() {
            return Err("No slave selected");
        }

        // Clear chip select (inactive)
        match self.cs_polarity {
            CsPolarity::ActiveLow => {
                self.generic.modify_reg(SpiRegister::Control1 as usize, |val| val | (1 << 3));
            }
            CsPolarity::ActiveHigh => {
                self.generic.modify_reg(SpiRegister::Control1 as usize, |val| val & !(1 << 3));
            }
        }

        self.generic.set_state(DeviceState::Ready);
        Ok(())
    }}

    /// Write and read a single byte on the SPI bus.
    ///
    /// # Safety
    /// A slave must be selected.
    pub unsafe fn transfer_byte(&self, write_byte: u8) -> Result<u8, &'static str> { unsafe {
        if !self.generic.is_enabled() {
            return Err("No slave selected");
        }

        // Write the byte to the data register
        self.generic.write_reg(SpiRegister::Data as usize, write_byte as u32);

        // Wait for transmit complete (bit 0 in status)
        if !self.generic.wait_bit_set(SpiRegister::Status as usize, 0, 1000) {
            return Err("SPI transfer timeout");
        }

        // Wait for receive not empty (bit 2 in status)
        if !self.generic.wait_bit_set(SpiRegister::Status as usize, 2, 1000) {
            return Err("SPI receive timeout");
        }

        // Read the received byte from data register
        let read_byte = self.generic.read_reg(SpiRegister::Data as usize) as u8;
        Ok(read_byte)
    }}

    /// Write multiple bytes on the SPI bus.
    ///
    /// # Safety
    /// A slave must be selected.
    pub unsafe fn write_bytes(&self, bytes: &[u8]) -> Result<(), &'static str> { unsafe {
        for &byte in bytes {
            let _ = self.transfer_byte(byte)?;
        }
        Ok(())
    }}

    /// Read multiple bytes from the SPI bus.
    ///
    /// # Safety
    /// A slave must be selected.
    pub unsafe fn read_bytes(&self, buffer: &mut [u8]) -> Result<(), &'static str> { unsafe {
        for byte in buffer.iter_mut() {
            *byte = self.transfer_byte(0xFF)?;
        }
        Ok(())
    }}

    /// Get the current device state.
    pub fn state(&self) -> DeviceState {
        self.generic.state()
    }

    /// Get the SPI mode.
    pub fn mode(&self) -> SpiMode {
        self.mode
    }

    /// Get the clock speed.
    pub fn speed(&self) -> SpiSpeed {
        self.speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spi_mode_values() {
        assert_eq!(SpiMode::Mode0 as u32, 0);
        assert_eq!(SpiMode::Mode1 as u32, 1);
        assert_eq!(SpiMode::Mode2 as u32, 2);
        assert_eq!(SpiMode::Mode3 as u32, 3);
    }

    #[test]
    fn test_spi_speed_values() {
        assert_eq!(SpiSpeed::Slow1MHz as u32, 1_000_000);
        assert_eq!(SpiSpeed::Fast25MHz as u32, 25_000_000);
    }

    #[test]
    fn test_spi_device_creation() {
        let _device: SpiDevice<0x40004000> = SpiDevice::new(
            SpiMode::Mode0,
            SpiSpeed::Fast25MHz,
            CsPolarity::ActiveLow,
        );
    }

    #[test]
    fn test_cs_polarity() {
        let low = CsPolarity::ActiveLow;
        let high = CsPolarity::ActiveHigh;
        assert_ne!(low, high);
    }
}
