//! Interrupt controller and architectural interrupt constants.

use super::{BitField32};

/// legacy 8259 PIC (Programmable Interrupt Controller) constants.
pub mod pic {
    pub const MASTER_CMD: u16 = 0x20;
    pub const MASTER_DATA: u16 = 0x21;
    pub const SLAVE_CMD: u16 = 0xA0;
    pub const SLAVE_DATA: u16 = 0xA1;

    pub const EOI: u8 = 0x20;

    // ICW1 Bits
    pub const ICW1_ICW4: u8 = 0x01;      // ICW4 (backup) needed
    pub const ICW1_SINGLE: u8 = 0x02;    // Single (cascade) mode
    pub const ICW1_INTERVAL4: u8 = 0x04; // Call address interval 4
    pub const ICW1_LEVEL: u8 = 0x08;     // Level triggered (edge) mode
    pub const ICW1_INIT: u8 = 0x10;      // Initialization bit

    // ICW4 Bits
    pub const ICW4_8086: u8 = 0x01;      // 8086/88 (MCS-80/85) mode
    pub const ICW4_AUTO: u8 = 0x02;      // Auto (normal) EOI
    pub const ICW4_BUF_SLAVE: u8 = 0x08;  // Buffered mode/slave
    pub const ICW4_BUF_MASTER: u8 = 0x0C; // Buffered mode/master
    pub const ICW4_SFNM: u8 = 0x10;      // Special fully nested mode
}

/// x86_64 Local APIC registers offsets and bits.
pub mod apic {
    use super::BitField32;

    pub const LAPIC_ID: usize = 0x020;
    pub const LAPIC_VER: usize = 0x030;
    pub const LAPIC_TPR: usize = 0x080;
    pub const LAPIC_APR: usize = 0x090;
    pub const LAPIC_PPR: usize = 0x0A0;
    pub const LAPIC_EOI: usize = 0x0B0;
    pub const LAPIC_RRD: usize = 0x0C0;
    pub const LAPIC_LDR: usize = 0x0D0;
    pub const LAPIC_DFR: usize = 0x0E0;
    pub const LAPIC_SVR: usize = 0x0F0;
    pub const LAPIC_ISR: usize = 0x100;
    pub const LAPIC_TMR: usize = 0x180;
    pub const LAPIC_IRR: usize = 0x200;
    pub const LAPIC_ESR: usize = 0x280;
    pub const LAPIC_ICR_LOW: usize = 0x300;
    pub const LAPIC_ICR_HIGH: usize = 0x310;
    pub const LAPIC_LVT_TIMER: usize = 0x320;
    pub const LAPIC_LVT_THERMAL: usize = 0x330;
    pub const LAPIC_LVT_PERF: usize = 0x340;
    pub const LAPIC_LVT_LINT0: usize = 0x350;
    pub const LAPIC_LVT_LINT1: usize = 0x360;
    pub const LAPIC_LVT_ERROR: usize = 0x370;
    pub const LAPIC_TICR: usize = 0x380;
    pub const LAPIC_TCCR: usize = 0x390;
    pub const LAPIC_TDCR: usize = 0x3E0;

    // SVR Bits
    pub const SVR_ENABLE: BitField32 = BitField32::new(1, 8);
    pub const SVR_VECTOR: BitField32 = BitField32::new(0xFF, 0);

    // Timer LVT Bits
    pub const LVT_TIMER_MODE_PERIODIC: BitField32 = BitField32::new(1, 17);
    pub const LVT_TIMER_MASK: BitField32 = BitField32::new(1, 16);
    pub const LVT_TIMER_VECTOR: BitField32 = BitField32::new(0xFF, 0);

    // ICR Bits
    pub const ICR_DEST_SHORTHAND: BitField32 = BitField32::new(0b11, 18);
    pub const ICR_DEST_ALL_EXCLUDING_SELF: u32 = 0b11;
    
    // Magic Values
    pub const LAPIC_DEFAULT_BASE: u64 = 0xFEE0_0000;
    pub const X2APIC_MSR_BASE: u32 = 0x800;
}

/// GIC (v2) registers offsets.
pub mod gic {
    pub const GICD_CTLR: usize = 0x000;
    pub const GICD_ISENABLER: usize = 0x100;
    pub const GICD_ICENABLER: usize = 0x180;
    pub const GICD_IPRIORITYR: usize = 0x400;
    pub const GICD_ITARGETSR: usize = 0x800;
    pub const GICD_ICFGR: usize = 0xC00;
    pub const GICD_PIDR2: usize = 0xFE8;

    pub const GICC_CTLR: usize = 0x000;
    pub const GICC_PMR: usize = 0x004;
    pub const GICC_IAR: usize = 0x00C;
    pub const GICC_EOIR: usize = 0x010;
}

/// x86_64 Specific Architectural Constants.
pub mod x86_64 {
    // Interrupt Stack Table (IST) Indices
    pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
    pub const PAGE_FAULT_IST_INDEX: u16 = 1;
    pub const SYSCALL_IST_INDEX: u16 = 2;

    // Stack sizes
    pub const IST_STACK_SIZE: usize = 4096 * 5;
}
