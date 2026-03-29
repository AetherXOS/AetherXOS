use crate::generated_consts::AARCH64_GIC_CPU_PRIORITY_MASK;
use crate::interfaces::InterruptController;
use core::ptr::{read_volatile, write_volatile};

/// INTID 1023 — the GIC's pseudo-interrupt for "no pending interrupt".
pub const GIC_SPURIOUS_INTID: u32 = 1023;
/// GICD_SGIR offset within the GIC Distributor register map.
pub const GICD_SGIR_OFFSET: usize = 0xF00;

pub struct Gic {
    gicd_base: usize,
    gicc_base: usize,
}

pub static GIC: spin::Mutex<Gic> = spin::Mutex::new(Gic::new(0, 0));

impl Gic {
    pub const fn new(gicd_base: usize, gicc_base: usize) -> Self {
        Self {
            gicd_base,
            gicc_base,
        }
    }

    pub fn update_bases(&mut self, gicd: usize, gicc: usize) {
        self.gicd_base = gicd;
        self.gicc_base = gicc;
    }

    /// Returns the GIC Distributor base address (needed for SGI broadcasts).
    pub fn gicd_base_addr(&self) -> usize {
        self.gicd_base
    }

    pub fn read_iar(&self) -> u32 {
        if self.gicc_base == 0 {
            return GIC_SPURIOUS_INTID;
        } // Spurious
        unsafe { read_volatile((self.gicc_base + 0x0C) as *const u32) }
    }
}

impl InterruptController for Gic {
    unsafe fn initialize(&mut self) {
        // Enable GIC Distributor (GICD_CTLR)
        write_volatile((self.gicd_base + 0x0) as *mut u32, 1);

        // Enable CPU Interface (GICC_CTLR)
        write_volatile((self.gicc_base + 0x0) as *mut u32, 1);

        // Priority mask configured via Cargo metadata (0-255).
        write_volatile(
            (self.gicc_base + 0x4) as *mut u32,
            AARCH64_GIC_CPU_PRIORITY_MASK.min(0xFF),
        );
    }

    unsafe fn enable_interrupt(&mut self, irq: u8) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ISENABLERn
        write_volatile(
            (self.gicd_base + 0x100 + reg as usize) as *mut u32,
            1 << bit,
        );
    }

    unsafe fn disable_interrupt(&mut self, irq: u8) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ICENABLERn
        write_volatile(
            (self.gicd_base + 0x180 + reg as usize) as *mut u32,
            1 << bit,
        );
    }

    unsafe fn end_of_interrupt(&mut self, irq: u8) {
        // GICC_EOIR
        write_volatile((self.gicc_base + 0x10) as *mut u32, irq as u32);
    }
}
