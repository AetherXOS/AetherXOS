//! AArch64 specific architectural constants and bitfields.

/// System Control Register (SCTLR_EL1) Bits.
pub mod sctlr {
    pub const M:   u64 = 1 << 0;  // MMU enable
    pub const A:   u64 = 1 << 1;  // Alignment check enable
    pub const C:   u64 = 1 << 2;  // Cacheability enable
    pub const SA:  u64 = 1 << 3;  // Stack alignment check enable
    pub const I:   u64 = 1 << 12; // Instruction cacheability enable
    pub const DZE: u64 = 1 << 14; // Dirty Zero Enable
    pub const UCT: u64 = 1 << 15; // User Access to CTR_EL0
    pub const UCI: u64 = 1 << 26; // User Access to Cache/IC/DC instructions
}

/// Translation Control Register (TCR_EL1) Bits.
pub mod tcr {
    pub const T0SZ_OFFSET: usize = 0;
    pub const T1SZ_OFFSET: usize = 16;
    pub const IRGN0_OFFSET: usize = 8;
    pub const ORGN0_OFFSET: usize = 10;
    pub const SH0_OFFSET: usize = 12;
    pub const TG0_OFFSET: usize = 14;
    
    pub const TG0_4KB: u64 = 0b00 << 14;
    pub const SH0_INNER: u64 = 0b11 << 12;
    pub const ORGN0_WBWA: u64 = 0b01 << 10;
    pub const IRGN0_WBWA: u64 = 0b01 << 8;
}

/// MAIR_EL1 Attribute Indices.
pub mod mair {
    pub const DEVICE_NGNRNE: u64 = 0x00;
    pub const NORMAL_WRITEBACK: u64 = 0xFF;
}
