pub use super::BitField64;
pub const PAGE_SIZE: usize = 4096;

pub mod generic {
    use crate::kernel::bit_utils::BitField64;

    pub const VALID: BitField64 = BitField64::new(1, 0);
    pub const TABLE: BitField64 = BitField64::new(1, 1);
    pub const USER: BitField64 = BitField64::new(1, 6);
    pub const READ_ONLY: BitField64 = BitField64::new(1, 7);
    pub const ACCESS_FLAG: BitField64 = BitField64::new(1, 10);
    
    // Address extraction mask (up to 48 bits for AArch64/x86_64)
    pub const PHYS_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

    #[inline(always)]
    pub fn get_phys_addr(entry: u64) -> u64 {
        entry & PHYS_ADDR_MASK
    }

    /// Extract indices for 4-level paging (e.g. AArch64 4KB, x86_64).
    #[inline(always)]
    pub fn get_indices(va: u64) -> [usize; 4] {
        use aethercore_common::bits::bit_range;
        [
            bit_range(va, 39, 47) as usize, // L0 / P4
            bit_range(va, 30, 38) as usize, // L1 / P3
            bit_range(va, 21, 29) as usize, // L2 / P2
            bit_range(va, 12, 20) as usize, // L3 / P1
        ]
    }
}

pub mod x86_64 {
    use aethercore_common::bits::BitField64;
    pub use super::PAGE_SIZE;
    pub const PRESENT: BitField64 = BitField64::new(1, 0);
    pub const WRITABLE: BitField64 = BitField64::new(1, 1);
    pub const USER: BitField64 = BitField64::new(1, 2);
    pub const WRITE_THROUGH: BitField64 = BitField64::new(1, 3);
    pub const CACHE_DISABLE: BitField64 = BitField64::new(1, 4);
    pub const ACCESSED: BitField64 = BitField64::new(1, 5);
    pub const DIRTY: BitField64 = BitField64::new(1, 6);
    pub const HUGE: BitField64 = BitField64::new(1, 7);
    pub const GLOBAL: BitField64 = BitField64::new(1, 8);
    pub const COW: BitField64 = BitField64::new(1, 9);
    pub const NO_EXECUTE: BitField64 = BitField64::new(1, 63);
}

pub use generic::*;
pub mod x86 {
    pub use super::x86_64::*;
}
