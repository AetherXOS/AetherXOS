//! x86_64 specific architectural constants and bitfields.

/// Interrupt Stack Table (IST) Indices.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const SYSCALL_IST_INDEX: u16 = 2;

/// Stack sizes for architectural handlers.
pub const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;
pub const PAGE_FAULT_STACK_SIZE: usize = 4096 * 5;

/// CR0 Register Bits.
pub mod cr0 {
    use aethercore_common::bits::BitField64;
    pub const PE: BitField64 = BitField64::new(1, 0);  // Protection Enable
    pub const MP: BitField64 = BitField64::new(1, 1);  // Monitor Coprocessor
    pub const EM: BitField64 = BitField64::new(1, 2);  // Emulation
    pub const TS: BitField64 = BitField64::new(1, 3);  // Task Switched
    pub const NE: BitField64 = BitField64::new(1, 5);  // Numeric Error
    pub const WP: BitField64 = BitField64::new(1, 16); // Write Protect
    pub const AM: BitField64 = BitField64::new(1, 18); // Alignment Mask
    pub const NW: BitField64 = BitField64::new(1, 29); // Not Write-through
    pub const CD: BitField64 = BitField64::new(1, 30); // Cache Disable
    pub const PG: BitField64 = BitField64::new(1, 31); // Paging
}

/// CR4 Register Bits.
pub mod cr4 {
    pub const VME: u64 = 1 << 0;  // Virtual-8086 Mode Extensions
    pub const PVI: u64 = 1 << 1;  // Protected-Mode Virtual Interrupts
    pub const TSD: u64 = 1 << 2;  // Time Stamp Disable
    pub const DE:  u64 = 1 << 3;  // Debugging Extensions
    pub const PSE: u64 = 1 << 4;  // Page Size Extensions
    pub const PAE: u64 = 1 << 5;  // Physical Address Extension
    pub const MCE: u64 = 1 << 6;  // Machine-Check Enable
    pub const PGE: u64 = 1 << 7;  // Page Global Enable
    pub const PCE: u64 = 1 << 8;  // Performance-Monitoring Counter Enable
    pub const OSFXSR: u64 = 1 << 9; // OS Support for FXSAVE/FXRSTOR
    pub const OSXMMEXCPT: u64 = 1 << 10; // OS Support for Unmasked SIMD Floating-Point Exceptions
    pub const UMIP: u64 = 1 << 11; // User-Mode Instruction Prevention
    pub const VMXE: u64 = 1 << 13; // VMX Enable
    pub const SMXE: u64 = 1 << 14; // SMX Enable
    pub const FSGSBASE: u64 = 1 << 16; // FSGSBASE Enable
    pub const PCIDE: u64 = 1 << 17; // PCID Enable
    pub const OSXSAVE: u64 = 1 << 18; // XSAVE and Processor Extended States Enable
    pub const SMEP: u64 = 1 << 20; // SMEP Enable
    pub const SMAP: u64 = 1 << 21; // SMAP Enable
    pub const PKE:  u64 = 1 << 22; // Protection Key Enable
}

/// EFER Register Bits.
pub mod efer {
    use aethercore_common::bits::BitField64;
    pub const SCE: BitField64 = BitField64::new(1, 0);  // System Call Extensions
    pub const LME: BitField64 = BitField64::new(1, 8);  // Long Mode Enable
    pub const LMA: BitField64 = BitField64::new(1, 10); // Long Mode Active
    pub const NXE: BitField64 = BitField64::new(1, 11); // No-Execute Enable
    pub const SVME: BitField64 = BitField64::new(1, 12); // Secure Virtual Machine Enable
    pub const LMSLE: BitField64 = BitField64::new(1, 13); // Long Mode Segment Limit Enable
    pub const FFXSR: BitField64 = BitField64::new(1, 14); // Fast FXSAVE/FXRSTOR
}
