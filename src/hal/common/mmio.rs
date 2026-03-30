//! Memory-Mapped I/O (MMIO) register abstractions.
//! Provides a safe and structured way to interact with hardware registers,
//! reducing boilerplate and pointer arithmetic across the HAL.

use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

/// A single MMIO register.
pub struct MmioRegister<T: Copy> {
    address: usize,
    _marker: PhantomData<T>,
}

impl<T: Copy> MmioRegister<T> {
    /// Create a new register from its base and offset.
    pub const fn new(base: usize, offset: usize) -> Self {
        Self {
            address: base + offset,
            _marker: PhantomData,
        }
    }

    /// Read the register value.
    /// Safety: Caller must ensure the register is valid and accessible.
    #[inline(always)]
    pub unsafe fn read(&self) -> T {
        unsafe { read_volatile(self.address as *const T) }
    }

    /// Write a value to the register.
    /// Safety: Caller must ensure the register is valid and accessible.
    #[inline(always)]
    pub unsafe fn write(&self, val: T) {
        unsafe { write_volatile(self.address as *mut T, val) }
    }

    /// Update the register value using a closure.
    /// Safety: Caller must ensure the register is valid and accessible.
    #[inline(always)]
    pub unsafe fn update<F>(&self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        unsafe {
            let val = self.read();
            self.write(f(val));
        }
    }
}

/// Bit-manipulation extensions for MMIO registers.
impl MmioRegister<u32> {
    #[inline(always)]
    pub unsafe fn set_bits(&self, bits: u32) {
        unsafe { self.update(|v| v | bits) };
    }

    #[inline(always)]
    pub unsafe fn clear_bits(&self, bits: u32) {
        unsafe { self.update(|v| v & !bits) };
    }

    #[inline(always)]
    pub unsafe fn has_bits(&self, bits: u32) -> bool {
        unsafe { (self.read() & bits) == bits }
    }

    /// Wait for a set of bits to become set.
    /// Safety: Caller must ensure the register is valid and accessible.
    pub unsafe fn wait_for_bits(&self, bits: u32, timeout_spins: Option<u32>) -> bool {
        let mut spins = 0;
        unsafe {
            while (self.read() & bits) != bits {
                if let Some(t) = timeout_spins {
                    spins += 1;
                    if spins >= t { return false; }
                }
                core::hint::spin_loop();
            }
        }
        true
    }
}

impl MmioRegister<u64> {
    #[inline(always)]
    pub unsafe fn set_bits(&self, bits: u64) {
        unsafe { self.update(|v| v | bits) };
    }

    #[inline(always)]
    pub unsafe fn clear_bits(&self, bits: u64) {
        unsafe { self.update(|v| v & !bits) };
    }

    #[inline(always)]
    pub unsafe fn has_bits(&self, bits: u64) -> bool {
        unsafe { (self.read() & bits) == bits }
    }
}

/// A structured MMIO block.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    #[inline(always)]
    pub fn reg<T: Copy>(&self, offset: usize) -> MmioRegister<T> {
        MmioRegister::new(self.base, offset)
    }

    #[inline(always)]
    pub fn base(&self) -> usize {
        self.base
    }
}

/// Helper for translating virtual addresses using HHDM.
pub fn virt_to_phys(addr: usize) -> Option<u64> {
    let hhdm = crate::hal::common::boot::hhdm_offset()?;
    let addr_u64 = addr as u64;
    if addr_u64 < hhdm { return None; }
    Some(addr_u64 - hhdm)
}

/// Helper for translating physical addresses to virtual using HHDM.
pub fn phys_to_virt(phys: u64) -> usize {
    let hhdm = crate::hal::common::boot::hhdm_offset().unwrap_or(0);
    (hhdm + phys) as usize
}

/// Read a u32 from physical address.
pub fn read_phys_u32(phys: u64) -> Option<u32> {
    let virt = phys_to_virt(phys);
    unsafe { Some(core::ptr::read_volatile(virt as *const u32)) }
}

/// Write a u32 to physical address.
pub fn write_phys_u32(phys: u64, val: u32) -> bool {
    let virt = phys_to_virt(phys);
    unsafe { core::ptr::write_volatile(virt as *mut u32, val) };
    true
}

/// Read a u64 from physical address.
pub fn read_phys_u64(phys: u64) -> Option<u64> {
    let virt = phys_to_virt(phys);
    unsafe { Some(core::ptr::read_volatile(virt as *const u64)) }
}

/// Write a u64 to physical address.
pub fn write_phys_u64(phys: u64, val: u64) -> bool {
    let virt = phys_to_virt(phys);
    unsafe { core::ptr::write_volatile(virt as *mut u64, val) };
    true
}
