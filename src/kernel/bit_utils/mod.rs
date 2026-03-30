//! Base structures for bitfield and register value manipulation.

/// A trait for bitfields and register values.
pub trait BitField {
    type Storage: Copy;
    fn mask() -> Self::Storage;
    fn shift() -> usize;
}

/// Helper for u32 bitfields.
#[derive(Debug, Clone, Copy)]
pub struct BitField32 {
    pub mask: u32,
    pub shift: u32,
}

impl BitField32 {
    pub const fn new(mask: u32, shift: u32) -> Self {
        Self { mask, shift }
    }

    #[inline(always)]
    pub const fn read(&self, val: u32) -> u32 {
        (val & (self.mask << self.shift)) >> self.shift
    }

    #[inline(always)]
    pub const fn write(&self, val: u32, field_val: u32) -> u32 {
        (val & !(self.mask << self.shift)) | ((field_val & self.mask) << self.shift)
    }

    #[inline(always)]
    pub const fn bit(&self, val: u32) -> bool {
        (val & (1 << self.shift)) != 0
    }

    #[inline(always)]
    pub const fn set_bit(&self, val: u32, set: bool) -> u32 {
        if set { val | (1 << self.shift) } else { val & !(1 << self.shift) }
    }
}

/// Helper for u64 bitfields (e.g. Page Table Entries).
#[derive(Debug, Clone, Copy)]
pub struct BitField64 {
    pub mask: u64,
    pub shift: u32,
}

impl BitField64 {
    pub const fn new(mask: u64, shift: u32) -> Self {
        Self { mask, shift }
    }

    #[inline(always)]
    pub const fn read(&self, val: u64) -> u64 {
        (val & (self.mask << self.shift)) >> self.shift
    }

    #[inline(always)]
    pub const fn write(&self, val: u64, field_val: u64) -> u64 {
        (val & !(self.mask << self.shift)) | ((field_val & self.mask) << self.shift)
    }

    #[inline(always)]
    pub const fn bit(&self, val: u64) -> bool {
        (val & (1 << self.shift)) != 0
    }

    #[inline(always)]
    pub const fn set_bit(&self, val: u64, set: bool) -> u64 {
        if set { val | (1 << self.shift) } else { val & !(1 << self.shift) }
    }
}

pub mod paging;
pub mod interrupts;
pub mod io;
pub mod x86_64_arch;
pub mod aarch64_arch;

pub use paging::*;
pub use interrupts::*;
pub use io::*;
pub use crate::kernel::bit_utils::io::{perf, com};

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_bitfield32_read_write() {
        let field = BitField32::new(0b11, 4);
        let val = 0u32;
        let new_val = field.write(val, 2);
        assert_eq!(new_val, 2 << 4);
        assert_eq!(field.read(new_val), 2);
    }

    #[test_case]
    fn test_bitfield32_bit() {
        let field = BitField32::new(1, 3);
        let val = 1 << 3;
        assert!(field.bit(val));
        assert!(!field.bit(0));
        let val2 = field.set_bit(0, true);
        assert_eq!(val2, 1 << 3);
        assert!(!field.bit(field.set_bit(val2, false)));
    }
}
