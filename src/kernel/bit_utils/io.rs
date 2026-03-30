//! I/O device registers and architectural control bits.

use super::{BitField32, BitField64};

/// PL011 UART registers bits.
pub mod pl011 {
    use super::BitField32;

    pub const FR_TXFF: BitField32 = BitField32::new(1, 5);
    pub const FR_RXFE: BitField32 = BitField32::new(1, 4);
    pub const FR_BUSY: BitField32 = BitField32::new(1, 3);
    pub const FR_TXFE_REAL: BitField32 = BitField32::new(1, 7);

    pub const CR_UARTEN: BitField32 = BitField32::new(1, 0);
    pub const CR_TXE: BitField32 = BitField32::new(1, 8);
    pub const CR_RXE: BitField32 = BitField32::new(1, 9);

    pub const LCR_FEN: BitField32 = BitField32::new(1, 4);
    pub const LCR_WLEN8: BitField32 = BitField32::new(0b11, 5);

    pub const INT_RX: BitField32 = BitField32::new(1, 4);
    pub const INT_TX: BitField32 = BitField32::new(1, 5);

    pub const UARTDR: usize = 0x000;
    pub const UARTFR: usize = 0x018;
    pub const UARTIBRD: usize = 0x024;
    pub const UARTFBRD: usize = 0x028;
    pub const UARTLCR_H: usize = 0x02C;
    pub const UARTCR: usize = 0x030;
    pub const UARTIMSC: usize = 0x038;
    pub const UARTMIS: usize = 0x040;
    pub const UARTICR: usize = 0x044;
}

/// x86_64 Standard COM Port (UART 16550A) constants.
pub mod com {
    pub const COM1_BASE: u16 = 0x3F8;
    
    pub const DATA: u16 = 0;
    pub const IER: u16 = 1;
    pub const IIR_FCR: u16 = 2;
    pub const LCR: u16 = 3;
    pub const MCR: u16 = 4;
    pub const LSR: u16 = 5;
    pub const MSR: u16 = 6;
    pub const SCR: u16 = 7;

    pub const LSR_DATA_READY: u8 = 1 << 0;
    pub const LSR_THRE: u8 = 1 << 5;

    pub const LCR_DLAB: u8 = 1 << 7;
    pub const LCR_8N1: u8 = 0x03;

    pub const FCR_ENABLE: u8 = 1 << 0;
    pub const FCR_CLEAR_RX: u8 = 1 << 1;
    pub const FCR_CLEAR_TX: u8 = 1 << 2;
    pub const FCR_DMA_MODE: u8 = 1 << 3;
    pub const FCR_64BYTE: u8 = 1 << 5;
    pub const FCR_TRIGGER_LEVEL_14: u8 = 0xC0;

    pub const MCR_DTR: u8 = 1 << 0;
    pub const MCR_RTS: u8 = 1 << 1;
    pub const MCR_OUT1: u8 = 1 << 2;
    pub const MCR_OUT2: u8 = 1 << 3;
    pub const MCR_LOOPBACK: u8 = 1 << 4;
}

/// PCI Configuration Space constants.
pub mod pci {
    pub const CONFIG_ADDR: u16 = 0xCF8;
    pub const CONFIG_DATA: u16 = 0xCFC;

    pub const VENDOR_DEVICE: u8 = 0x00;
    pub const CLASS_SUBCLASS: u8 = 0x08;
    pub const HEADER_TYPE: u8 = 0x0C;
    pub const BAR0: u8 = 0x10;
    pub const INTERRUPT_LINE: u8 = 0x3C;

    pub const HEADER_TYPE_MULTIFUNCTION: u8 = 0x80;
}

/// x86_64 Performance and Power management constants.
pub mod perf {
    pub const IA32_PERF_CTL: u32 = 0x199;
    pub const RATIO_HIGH: u64 = 0x20;
    pub const RATIO_BALANCED: u64 = 0x18;
    pub const RATIO_POWERSAVE: u64 = 0x08;
}

/// AArch64 System Timer bits (CNTP_CTL_EL1).
pub mod timer {
    use super::BitField64;
    pub const ENABLE: BitField64 = BitField64::new(1, 0);
    pub const IMASK: BitField64 = BitField64::new(1, 1);
    pub const ISTATUS: BitField64 = BitField64::new(1, 2);
}

/// AArch64 Exception registers bits.
pub mod aarch64_sys {
    use super::BitField32;
    pub const ESR_EC: BitField32 = BitField32::new(0x3F, 26);
    pub const ESR_IL: BitField32 = BitField32::new(1, 25);
    pub const ESR_ISS: BitField32 = BitField32::new(0x01FF_FFFF, 0);

    pub const EC_UNKNOWN: u32 = 0x00;
    pub const EC_SVC64: u32 = 0x15;
    pub const EC_IABORT_LOWER: u32 = 0x20;
    pub const EC_IABORT_CUR: u32 = 0x21;
    pub const EC_DABORT_LOWER: u32 = 0x24;
    pub const EC_DABORT_CUR: u32 = 0x25;
    pub const EC_SERROR: u32 = 0x2F;
}
