pub use super::BitField64;
pub const PAGE_SIZE: usize = 4096;

pub mod generic {
    use super::{BitField64, PAGE_SIZE};

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
        [
            ((va >> 39) & 0x1FF) as usize, // L0 / P4
            ((va >> 30) & 0x1FF) as usize, // L1 / P3
            ((va >> 21) & 0x1FF) as usize, // L2 / P2
            ((va >> 12) & 0x1FF) as usize, // L3 / P1
        ]
    }
}

pub mod x86_64 {
    pub use super::PAGE_SIZE;
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const COW: u64 = 1 << 9;
    pub const NO_EXECUTE: u64 = 1 << 63;
}

pub use generic::*;
pub mod x86 {
    pub use super::x86_64::*;
}
