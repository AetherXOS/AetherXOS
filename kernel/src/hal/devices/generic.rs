/// Generic memory-mapped device trait and utilities.
///
/// Provides a common interface for all MMIO devices, enabling:
/// - Safe register access patterns
/// - Interrupt handling abstractions
/// - Device lifecycle management (init/enable/disable)

use core::marker::PhantomData;

/// A register descriptor with type and offset information.
#[derive(Debug, Clone, Copy)]
pub struct RegisterDescriptor {
    /// Byte offset from the device base address.
    pub offset: usize,
    /// Size of the register in bytes.
    pub size: usize,
}

impl RegisterDescriptor {
    /// Create a new register descriptor.
    pub const fn new(offset: usize, size: usize) -> Self {
        RegisterDescriptor { offset, size }
    }

    /// Common register sizes.
    pub const fn u8() -> usize { 1 }
    pub const fn u16() -> usize { 2 }
    pub const fn u32() -> usize { 4 }
    pub const fn u64() -> usize { 8 }
}

/// Device state enum for lifecycle tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device not yet initialized.
    Uninitialized,
    /// Device initialized and ready.
    Ready,
    /// Device actively enabled/running.
    Enabled,
    /// Device disabled/suspended.
    Disabled,
    /// Device in error state.
    Error,
}

/// Generic trait for all MMIO devices.
///
/// Provides a unified interface for device initialization, control, and status checking.
pub trait MmioDevice {
    /// The base memory address of this device.
    fn base_address(&self) -> usize;

    /// Get the current state of the device.
    fn state(&self) -> DeviceState;

    /// Initialize the device. Transitions from Uninitialized → Ready.
    ///
    /// # Safety
    /// The caller must ensure that the base address is valid and the device
    /// is not already initialized.
    unsafe fn init(&mut self) -> Result<(), &'static str>;

    /// Enable the device. Transitions from Ready → Enabled.
    ///
    /// # Safety
    /// The device must be in Ready state.
    unsafe fn enable(&mut self) -> Result<(), &'static str>;

    /// Disable the device. Transitions from Enabled → Disabled.
    unsafe fn disable(&mut self) -> Result<(), &'static str>;

    /// Reset the device to Uninitialized state.
    unsafe fn reset(&mut self) -> Result<(), &'static str>;

    /// Read a register at the given offset as a u32.
    ///
    /// # Safety
    /// The offset must be valid for this device.
    unsafe fn read_register(&self, offset: usize) -> u32;

    /// Write a register at the given offset with a u32 value.
    ///
    /// # Safety
    /// The offset must be valid and the value appropriate for this device.
    unsafe fn write_register(&self, offset: usize, value: u32);

    /// Check if the device is in a ready state (Ready or Enabled).
    fn is_ready(&self) -> bool {
        matches!(self.state(), DeviceState::Ready | DeviceState::Enabled)
    }

    /// Check if the device is enabled.
    fn is_enabled(&self) -> bool {
        self.state() == DeviceState::Enabled
    }

    /// Check if the device is in error state.
    fn is_error(&self) -> bool {
        self.state() == DeviceState::Error
    }
}

/// A generic MMIO device with state tracking and lifecycle management.
///
/// # Type Parameters
/// - `BASE`: The physical memory base address of the device.
/// - `T`: Device-specific type tag for distinguishing different device families.
pub struct GenericMmioDevice<const BASE: usize, T> {
    state: DeviceState,
    _phantom: PhantomData<T>,
}

impl<const BASE: usize, T> GenericMmioDevice<BASE, T> {
    /// Create a new MMIO device in uninitialized state.
    pub const fn new() -> Self {
        GenericMmioDevice {
            state: DeviceState::Uninitialized,
            _phantom: PhantomData,
        }
    }

    /// Get the current device state.
    pub fn state(&self) -> DeviceState {
        self.state
    }

    /// Set the device state (used internally by device-specific code).
    pub(crate) fn set_state(&mut self, state: DeviceState) {
        self.state = state;
    }

    /// Read a register at the given offset as a u32.
    ///
    /// # Safety
    /// The offset must point to a valid, readable register.
    pub unsafe fn read_reg(&self, offset: usize) -> u32 {
        let addr = (BASE + offset) as *const u32;
        unsafe { core::ptr::read_volatile(addr) }
    }

    /// Write a register at the given offset with a u32 value.
    ///
    /// # Safety
    /// The offset must point to a valid, writable register.
    pub unsafe fn write_reg(&self, offset: usize, value: u32) {
        let addr = (BASE + offset) as *mut u32;
        unsafe { core::ptr::write_volatile(addr, value); }
    }

    /// Read a register as u64 (for 64-bit registers).
    ///
    /// # Safety
    /// The offset must point to a valid 64-bit register.
    pub unsafe fn read_reg64(&self, offset: usize) -> u64 {
        let addr = (BASE + offset) as *const u64;
        unsafe { core::ptr::read_volatile(addr) }
    }

    /// Write a register as u64.
    ///
    /// # Safety
    /// The offset must point to a valid 64-bit writable register.
    pub unsafe fn write_reg64(&self, offset: usize, value: u64) {
        let addr = (BASE + offset) as *mut u64;
        unsafe { core::ptr::write_volatile(addr, value); }
    }

    /// Read a register as u8.
    ///
    /// # Safety
    /// The offset must point to a valid 8-bit register.
    pub unsafe fn read_reg8(&self, offset: usize) -> u8 {
        let addr = (BASE + offset) as *const u8;
        unsafe { core::ptr::read_volatile(addr) }
    }

    /// Write a register as u8.
    ///
    /// # Safety
    /// The offset must point to a valid 8-bit writable register.
    pub unsafe fn write_reg8(&self, offset: usize, value: u8) {
        let addr = (BASE + offset) as *mut u8;
        unsafe { core::ptr::write_volatile(addr, value); }
    }

    /// Modify a register using a read-modify-write pattern.
    ///
    /// Useful for setting/clearing specific bits without affecting others.
    ///
    /// # Safety
    /// The offset must point to a valid register.
    pub unsafe fn modify_reg<F>(&self, offset: usize, f: F)
    where
        F: FnOnce(u32) -> u32,
    { unsafe {
        let current = self.read_reg(offset);
        let modified = f(current);
        self.write_reg(offset, modified);
    }}

    /// Read a specific field from a register using a BitField descriptor.
    ///
    /// # Safety
    /// The offset in the BitField must be valid.
    pub unsafe fn read_field(&self, offset: usize, field: crate::kernel::bit_utils::BitField32) -> u32 { unsafe {
        field.read(self.read_reg(offset))
    }}

    /// Write a specific field to a register without affecting other bits.
    ///
    /// # Safety
    /// The offset in the BitField must be valid.
    pub unsafe fn write_field(&self, offset: usize, field: crate::kernel::bit_utils::BitField32, val: u32) { unsafe {
        self.modify_reg(offset, |reg| field.write(reg, val));
    }}

    /// Wait for a register bit to become set.
    ///
    /// Polls the register until the specified bit is 1 or timeout occurs.
    pub fn wait_bit_set(&self, offset: usize, bit: u32, max_spins: u32) -> bool {
        let mask = 1u32 << bit;
        for _ in 0..max_spins {
            // SAFETY: This is a read-only operation; safe after init.
            unsafe {
                if (self.read_reg(offset) & mask) != 0 {
                    return true;
                }
            }
            // Brief spin
            unsafe {
                core::arch::asm!("nop");
            }
        }
        false
    }

    /// Wait for a register bit to become clear.
    pub fn wait_bit_clear(&self, offset: usize, bit: u32, max_spins: u32) -> bool {
        let mask = 1u32 << bit;
        for _ in 0..max_spins {
            // SAFETY: Read-only; safe after init.
            unsafe {
                if (self.read_reg(offset) & mask) == 0 {
                    return true;
                }
            }
            unsafe {
                core::arch::asm!("nop");
            }
        }
        false
    }
}

impl<const BASE: usize, T> MmioDevice for GenericMmioDevice<BASE, T> {
    fn base_address(&self) -> usize {
        BASE
    }

    fn state(&self) -> DeviceState {
        self.state
    }

    unsafe fn init(&mut self) -> Result<(), &'static str> {
        if self.state != DeviceState::Uninitialized {
            return Err("Device already initialized");
        }
        self.state = DeviceState::Ready;
        Ok(())
    }

    unsafe fn enable(&mut self) -> Result<(), &'static str> {
        if self.state != DeviceState::Ready {
            return Err("Device not in Ready state");
        }
        self.state = DeviceState::Enabled;
        Ok(())
    }

    unsafe fn disable(&mut self) -> Result<(), &'static str> {
        if self.state != DeviceState::Enabled {
            return Err("Device not enabled");
        }
        self.state = DeviceState::Disabled;
        Ok(())
    }

    unsafe fn reset(&mut self) -> Result<(), &'static str> {
        self.state = DeviceState::Uninitialized;
        Ok(())
    }

    unsafe fn read_register(&self, offset: usize) -> u32 { unsafe {
        self.read_reg(offset)
    }}

    unsafe fn write_register(&self, offset: usize, value: u32) { unsafe {
        self.write_reg(offset, value);
    }}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_state_transitions() {
        let mut device: GenericMmioDevice<0x1000, ()> = GenericMmioDevice::new();
        assert_eq!(device.state(), DeviceState::Uninitialized);

        unsafe {
            device.init().unwrap();
            assert_eq!(device.state(), DeviceState::Ready);

            device.enable().unwrap();
            assert_eq!(device.state(), DeviceState::Enabled);

            device.disable().unwrap();
            assert_eq!(device.state(), DeviceState::Disabled);

            device.reset().unwrap();
            assert_eq!(device.state(), DeviceState::Uninitialized);
        }
    }

    #[test]
    fn test_register_descriptor() {
        let rd = RegisterDescriptor::new(0x00, 4);
        assert_eq!(rd.offset, 0x00);
        assert_eq!(rd.size, 4);
    }

    #[test]
    fn test_device_is_ready() {
        let mut device: GenericMmioDevice<0x1000, ()> = GenericMmioDevice::new();
        assert!(!device.is_ready());

        unsafe {
            device.init().unwrap();
            assert!(device.is_ready());

            device.enable().unwrap();
            assert!(device.is_ready());
        }
    }
}
