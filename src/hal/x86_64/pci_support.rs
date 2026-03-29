pub(super) const CONFIG_ENABLE: u32 = 0x8000_0000;

#[inline(always)]
pub(super) fn config_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    CONFIG_ENABLE
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
}

#[inline(always)]
pub(super) fn byte_shift(offset: u8) -> u32 {
    ((offset & 3) * 8) as u32
}

#[inline(always)]
pub(super) fn word_shift(offset: u8) -> u32 {
    ((offset & 2) * 8) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn config_address_masks_lower_offset_bits() {
        let aligned = config_address(0x12, 0x03, 0x04, 0x10);
        let unaligned = config_address(0x12, 0x03, 0x04, 0x13);
        assert_eq!(aligned, unaligned);
        assert_eq!(aligned, 0x8012_1c10);
    }

    #[test_case]
    fn byte_and_word_shifts_follow_pci_encoding() {
        assert_eq!(byte_shift(0), 0);
        assert_eq!(byte_shift(3), 24);
        assert_eq!(word_shift(0), 0);
        assert_eq!(word_shift(2), 16);
        assert_eq!(word_shift(3), 16);
    }
}
