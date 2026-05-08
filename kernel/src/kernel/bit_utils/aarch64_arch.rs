//! AArch64 specific architectural constants and bitfields.

/// System Control Register (SCTLR_EL1) Bits.
pub mod sctlr {
    use aethercore_common::bits::BitField64;
    pub const M:   BitField64 = BitField64::new(1, 0);  // MMU enable
    pub const A:   BitField64 = BitField64::new(1, 1);  // Alignment check enable
    pub const C:   BitField64 = BitField64::new(1, 2);  // Cacheability enable
    pub const SA:  BitField64 = BitField64::new(1, 3);  // Stack alignment check enable
    pub const I:   BitField64 = BitField64::new(1, 12); // Instruction cacheability enable
    pub const DZE: BitField64 = BitField64::new(1, 14); // Dirty Zero Enable
    pub const UCT: BitField64 = BitField64::new(1, 15); // User Access to CTR_EL0
    pub const UCI: BitField64 = BitField64::new(1, 26); // User Access to Cache/IC/DC instructions
}

/// Translation Control Register (TCR_EL1) Bits.
pub mod tcr {
    use aethercore_common::bits::BitField64;
    pub const T0SZ_OFFSET: usize = 0;
    pub const T1SZ_OFFSET: usize = 16;
    pub const IRGN0_OFFSET: usize = 8;
    pub const ORGN0_OFFSET: usize = 10;
    pub const SH0_OFFSET: usize = 12;
    pub const TG0_OFFSET: usize = 14;
    
    pub const TG0_4KB: BitField64 = BitField64::new(0b11, 14);
    pub const SH0_INNER: BitField64 = BitField64::new(0b11, 12);
    pub const ORGN0_WBWA: BitField64 = BitField64::new(0b11, 10);
    pub const IRGN0_WBWA: BitField64 = BitField64::new(0b11, 8);
}

/// MAIR_EL1 Attribute Indices.
pub mod mair {
    pub const DEVICE_NGNRNE: u64 = 0x00;
    pub const NORMAL_WRITEBACK: u64 = 0xFF;
}
