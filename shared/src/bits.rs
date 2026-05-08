//! Bit manipulation and alignment utilities.

/// Align `addr` up to the next `align` boundary. `align` must be a power of two.
#[inline(always)]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// Align `addr` down to the previous `align` boundary. `align` must be a power of two.
#[inline(always)]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Check if `addr` is aligned to `align`. `align` must be a power of two.
#[inline(always)]
pub const fn is_aligned(addr: u64, align: u64) -> bool {
    addr & (align - 1) == 0
}

/// Create a bitmask for a range of bits `[start, end]`.
#[inline(always)]
pub const fn bit_mask(start: u8, end: u8) -> u64 {
    let len = end - start + 1;
    let mask = if len == 64 { !0 } else { (1 << len) - 1 };
    mask << start
}

/// Get a range of bits from `value`.
#[inline(always)]
pub const fn bit_range(value: u64, start: u8, end: u8) -> u64 {
    (value & bit_mask(start, end)) >> start
}

/// Set a range of bits in `value` to `bits`.
#[inline(always)]
pub const fn set_bit_range(value: u64, start: u8, end: u8, bits: u64) -> u64 {
    let mask = bit_mask(start, end);
    (value & !mask) | ((bits << start) & mask)
}


pub trait BitField {
    type Storage: Copy;
    fn mask() -> Self::Storage;
    fn shift() -> usize;
}

macro_rules! impl_bitfield {
    ($name:ident, $ty:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            pub mask: $ty,
            pub shift: u32,
        }

        impl $name {
            pub const fn new(mask: $ty, shift: u32) -> Self {
                Self { mask, shift }
            }

            #[inline(always)]
            pub const fn read(&self, val: $ty) -> $ty {
                (val & (self.mask << self.shift)) >> self.shift
            }

            #[inline(always)]
            pub const fn write(&self, val: $ty, field_val: $ty) -> $ty {
                (val & !(self.mask << self.shift)) | ((field_val & self.mask) << self.shift)
            }

            #[inline(always)]
            pub const fn bit(&self, val: $ty) -> bool {
                (val & (1 << self.shift)) != 0
            }

            #[inline(always)]
            pub const fn set_bit(&self, val: $ty, set: bool) -> $ty {
                if set { val | (1 << self.shift) } else { val & !(1 << self.shift) }
            }

            #[inline(always)]
            pub const fn extract(&self, val: $ty) -> $ty {
                (val >> self.shift) & self.mask
            }

            #[inline(always)]
            pub const fn replace(&self, val: $ty, field_val: $ty) -> $ty {
                (val & !(self.mask << self.shift)) | ((field_val & self.mask) << self.shift)
            }

            #[inline(always)]
            pub const fn clear(&self, val: $ty) -> $ty {
                val & !(self.mask << self.shift)
            }
        }
    };
}

impl_bitfield!(BitField8, u8);
impl_bitfield!(BitField16, u16);
impl_bitfield!(BitField32, u32);
impl_bitfield!(BitField64, u64);
