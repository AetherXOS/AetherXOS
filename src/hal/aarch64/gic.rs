use crate::generated_consts::AARCH64_GIC_CPU_PRIORITY_MASK;
use crate::interfaces::InterruptController;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// INTID 1023 — the GIC's pseudo-interrupt for "no pending interrupt".
pub const GIC_SPURIOUS_INTID: u32 = 1023;
/// GICD_SGIR offset within the GIC Distributor register map.
pub const GICD_SGIR_OFFSET: usize = 0xF00;

/// Snapshot of the runtime-visible GIC state used by platform/virt profiles.
#[derive(Debug, Clone, Copy)]
pub struct GicStats {
    pub initialized: bool,
    pub version: u32,
}

static GIC_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GIC_VERSION: AtomicU32 = AtomicU32::new(0);

pub struct Gic {
    gicd_base: usize,
    gicc_base: usize,
}

pub static GIC: spin::Mutex<Gic> = spin::Mutex::new(Gic::new(0, 0));

pub fn stats() -> GicStats {
    GicStats {
        initialized: GIC_INITIALIZED.load(Ordering::Relaxed),
        version: GIC_VERSION.load(Ordering::Relaxed),
    }
}

fn detect_gic_version(gicd_base: usize) -> u32 {
    if gicd_base == 0 {
        return 0;
    }

    // GICD_PIDR2[7:4] encodes GIC architecture revision.
    let pidr2 = unsafe { read_volatile((gicd_base + 0xFE8) as *const u32) };
    (pidr2 >> 4) & 0x0F
}

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
        GIC_INITIALIZED.store(false, Ordering::Relaxed);
        GIC_VERSION.store(detect_gic_version(gicd), Ordering::Relaxed);
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
        unsafe { write_volatile((self.gicd_base + 0x0) as *mut u32, 1) };

        // Enable CPU Interface (GICC_CTLR)
        unsafe { write_volatile((self.gicc_base + 0x0) as *mut u32, 1) };

        // Priority mask configured via Cargo metadata (0-255).
        unsafe {
            write_volatile(
                (self.gicc_base + 0x4) as *mut u32,
                AARCH64_GIC_CPU_PRIORITY_MASK.min(0xFF),
            )
        };

        GIC_INITIALIZED.store(true, Ordering::Relaxed);
        let detected_version = detect_gic_version(self.gicd_base);
        if detected_version != 0 {
            GIC_VERSION.store(detected_version, Ordering::Relaxed);
        }
    }

    unsafe fn enable_interrupt(&mut self, irq: u8) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ISENABLERn
        unsafe {
            write_volatile(
                (self.gicd_base + 0x100 + reg as usize) as *mut u32,
                1 << bit,
            )
        };
    }

    unsafe fn disable_interrupt(&mut self, irq: u8) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ICENABLERn
        unsafe {
            write_volatile(
                (self.gicd_base + 0x180 + reg as usize) as *mut u32,
                1 << bit,
            )
        };
    }

    unsafe fn end_of_interrupt(&mut self, irq: u8) {
        // GICC_EOIR
        unsafe { write_volatile((self.gicc_base + 0x10) as *mut u32, irq as u32) };
    }
}
