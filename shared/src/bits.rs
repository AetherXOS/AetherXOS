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
