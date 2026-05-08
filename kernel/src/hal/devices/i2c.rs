/// I2C (Inter-Integrated Circuit) device abstraction.
///
/// Provides a type-safe interface for I2C bus communication with
/// const-generic base addresses and state tracking.

use super::generic::{GenericMmioDevice, DeviceState, MmioDevice};

/// An I2C bus address (7-bit or 10-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cAddress(pub u16);

impl I2cAddress {
    /// Create a new 7-bit I2C address.
    pub const fn new(addr: u8) -> Self {
        I2cAddress((addr as u16) & 0x7F)
    }

    /// Create a new 10-bit I2C address.
    pub const fn new_10bit(addr: u16) -> Self {
        I2cAddress(addr & 0x3FF)
    }

    /// Get the address value.
    pub const fn value(&self) -> u16 {
        self.0
    }
}

/// I2C register offsets (common across most I2C controllers).
#[repr(usize)]
pub enum I2cRegister {
    /// Control Register
    Control = 0x00,
    /// Status Register
    Status = 0x04,
    /// Address Register
    Address = 0x08,
    /// Data Register
    Data = 0x0C,
    /// Clock Control Register
    ClockControl = 0x10,
    /// Interrupt Enable Register
    IntEnable = 0x14,
    /// Interrupt Status Register
    IntStatus = 0x18,
}

// I2C Control Register BitFields
pub const I2C_CTRL_ENABLE: crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 0);
pub const I2C_CTRL_ACK:    crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 2);
pub const I2C_CTRL_WRITE:  crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 4);
pub const I2C_CTRL_START:  crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 5);
pub const I2C_CTRL_STOP:   crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 6);

// I2C Status Register BitFields
pub const I2C_STAT_BUSY:   crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 0);
pub const I2C_STAT_TX_CMP: crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 1);
pub const I2C_STAT_RX_RDY: crate::kernel::bit_utils::BitField32 = crate::kernel::bit_utils::BitField32::new(1, 2);

/// I2C bus speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cSpeed {
    /// 100 kHz (Standard Mode)
    Standard = 100_000,
    /// 400 kHz (Fast Mode)
    Fast = 400_000,
    /// 1 MHz (Fast Mode Plus)
    FastPlus = 1_000_000,
    /// 3.4 MHz (High Speed Mode)
    HighSpeed = 3_400_000,
}

/// Generic I2C device with const-generic MMIO base address.
///
/// # Type Parameters
/// - `BASE`: The memory-mapped base address of the I2C controller.
pub struct I2cDevice<const BASE: usize> {
    generic: GenericMmioDevice<BASE, I2cDeviceTag>,
    speed: I2cSpeed,
    target_address: I2cAddress,
}

/// Type tag for I2C devices (used for type-level distinction).
pub struct I2cDeviceTag;

impl<const BASE: usize> I2cDevice<BASE> {
    /// Create a new I2C device in uninitialized state.
    ///
    /// # Arguments
    /// - `speed`: The I2C bus speed.
    /// - `target_address`: The default address for this device.
    pub const fn new(speed: I2cSpeed, target_address: I2cAddress) -> Self {
        I2cDevice {
            generic: GenericMmioDevice::new(),
            speed,
            target_address,
        }
    }

    /// Initialize the I2C bus at the specified speed.
    ///
    /// # Safety
    /// BASE must point to a valid I2C controller.
    pub unsafe fn init(&mut self) -> Result<(), &'static str> { unsafe {
        self.generic.init()?;
        
        // Program clock divider based on speed
        let divider = match self.speed {
            I2cSpeed::Standard => 100,
            I2cSpeed::Fast => 25,
            I2cSpeed::FastPlus => 10,
            I2cSpeed::HighSpeed => 3,
        };
        
        self.generic.write_reg(I2cRegister::ClockControl as usize, divider);
        
        // Enable I2C controller
        self.generic.write_field(I2cRegister::Control as usize, I2C_CTRL_ENABLE, 1);
        
        self.generic.set_state(DeviceState::Ready);
        Ok(())
    }}

    /// Start an I2C transaction.
    ///
    /// # Safety
    /// Device must be initialized and enabled.
    pub unsafe fn start(&mut self, address: I2cAddress, read: bool) -> Result<(), &'static str> { unsafe {
        if !self.generic.is_ready() {
            return Err("I2C device not ready");
        }

        // Set target address in address register
        let addr_val = (address.value() << 1) | (if read { 1 } else { 0 });
        self.generic.write_reg(I2cRegister::Address as usize, addr_val as u32);
        
        // Set START condition
        self.generic.write_field(I2cRegister::Control as usize, I2C_CTRL_START, 1);
        
        // Wait for bus to become busy
        if !self.generic.wait_bit_set(I2cRegister::Status as usize, I2C_STAT_BUSY.shift, 1000) {
            return Err("I2C START timeout");
        }

        Ok(())
    }}

    /// Send a byte on the I2C bus.
    ///
    /// # Safety
    /// A transaction must be in progress.
    pub unsafe fn write_byte(&self, byte: u8) -> Result<(), &'static str> { unsafe {
        // Write data to data register
        self.generic.write_reg(I2cRegister::Data as usize, byte as u32);
        
        // Set ACK and write bit in control
        self.generic.modify_reg(I2cRegister::Control as usize, |val| {
            let val = I2C_CTRL_ACK.write(val, 1);
            I2C_CTRL_WRITE.write(val, 1)
        });
        
        // Wait for transmission complete
        if !self.generic.wait_bit_set(I2cRegister::Status as usize, I2C_STAT_TX_CMP.shift, 1000) {
            return Err("I2C write timeout");
        }

        Ok(())
    }}

    /// Read a byte from the I2C bus.
    ///
    /// # Safety
    /// A transaction must be in progress.
    pub unsafe fn read_byte(&self, ack: bool) -> Result<u8, &'static str> { unsafe {
        // Set read bit and ACK if needed
        self.generic.modify_reg(I2cRegister::Control as usize, |val| {
            let val = I2C_CTRL_WRITE.write(val, 1);
            I2C_CTRL_ACK.write(val, if ack { 1 } else { 0 })
        });
        
        // Wait for data available
        if !self.generic.wait_bit_set(I2cRegister::Status as usize, I2C_STAT_RX_RDY.shift, 1000) {
            return Err("I2C read timeout");
        }

        // Read from data register
        let byte = self.generic.read_reg(I2cRegister::Data as usize) as u8;
        Ok(byte)
    }}

    /// Stop the I2C transaction.
    ///
    /// # Safety
    /// A transaction must be in progress.
    pub unsafe fn stop(&mut self) -> Result<(), &'static str> { unsafe {
        // Set STOP condition
        self.generic.write_field(I2cRegister::Control as usize, I2C_CTRL_STOP, 1);
        
        // Wait for bus to become idle
        if !self.generic.wait_bit_clear(I2cRegister::Status as usize, I2C_STAT_BUSY.shift, 1000) {
            return Err("I2C STOP timeout");
        }

        self.generic.set_state(DeviceState::Ready);
        Ok(())
    }}

    /// Get the current device state.
    pub fn state(&self) -> DeviceState {
        self.generic.state()
    }

    /// Get the bus speed.
    pub fn speed(&self) -> I2cSpeed {
        self.speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2c_address() {
        let addr = I2cAddress::new(0x50);
        assert_eq!(addr.value(), 0x50);

        let addr_10 = I2cAddress::new_10bit(0x300);
        assert_eq!(addr_10.value(), 0x300);
    }

    #[test]
    fn test_i2c_speed() {
        assert_eq!(I2cSpeed::Standard as u32, 100_000);
        assert_eq!(I2cSpeed::Fast as u32, 400_000);
    }

    #[test]
    fn test_i2c_device_creation() {
        let _device: I2cDevice<0x40005000> = I2cDevice::new(I2cSpeed::Fast, I2cAddress::new(0x50));
    }
}
