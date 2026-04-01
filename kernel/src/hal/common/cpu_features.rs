#[inline(always)]
pub fn has_bit_u32(value: u32, bit: u32) -> bool {
    (value & (1 << bit)) != 0
}

#[inline(always)]
pub fn field_u64(value: u64, shift: u32, mask: u64) -> u64 {
    (value >> shift) & mask
}

#[inline(always)]
pub fn field_present_u64(value: u64, shift: u32, mask: u64, absent_value: u64) -> bool {
    field_u64(value, shift, mask) != absent_value
}

#[inline(always)]
pub fn field_at_least_u64(value: u64, shift: u32, mask: u64, minimum: u64) -> bool {
    field_u64(value, shift, mask) >= minimum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn bit_and_field_helpers_decode_consistently() {
        assert!(has_bit_u32(0b1000, 3));
        assert!(!has_bit_u32(0b1000, 2));
        assert_eq!(field_u64(0xAB00, 8, 0xFF), 0xAB);
        assert!(field_present_u64(0x10, 4, 0xF, 0xF));
        assert!(field_at_least_u64(0x30, 4, 0xF, 3));
    }
}
