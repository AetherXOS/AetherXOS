use crate::generated_consts::AARCH64_GIC_CPU_PRIORITY_MASK;
use crate::interfaces::InterruptController;
use crate::kernel::bit_utils::gic as bits;
use core::ptr::read_volatile;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::hal::common::mmio::MmioBlock;

/// INTID 1023 — the GIC's pseudo-interrupt for "no pending interrupt".
pub const GIC_SPURIOUS_INTID: u32 = 1023;

/// Snapshot of the runtime-visible GIC state used by platform/virt profiles.
#[derive(Debug, Clone, Copy)]
pub struct GicStats {
    pub initialized: bool,
    pub version: u32,
}

static GIC_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GIC_VERSION: AtomicU32 = AtomicU32::new(0);

pub struct Gic {
    dist_block: MmioBlock,
    cpu_block: MmioBlock,
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
    // Safety: Caller must ensure gicd_base is valid.
    let pidr2 = unsafe { read_volatile((gicd_base + bits::GICD_PIDR2) as *const u32) };
    (pidr2 >> 4) & 0x0F
}

impl Gic {
    pub const fn new(dist_base: usize, cpu_base: usize) -> Self {
        Self {
            dist_block: MmioBlock::new(dist_base),
            cpu_block: MmioBlock::new(cpu_base),
        }
    }

    #[inline(always)]
    unsafe fn write_dist(&self, offset: usize, val: u32) {
        self.dist_block.reg::<u32>(offset).write(val);
    }

    #[inline(always)]
    unsafe fn read_dist(&self, offset: usize) -> u32 {
        self.dist_block.reg::<u32>(offset).read()
    }

    #[inline(always)]
    unsafe fn write_cpu(&self, offset: usize, val: u32) {
        self.cpu_block.reg::<u32>(offset).write(val);
    }

    #[inline(always)]
    unsafe fn read_cpu(&self, offset: usize) -> u32 {
        self.cpu_block.reg::<u32>(offset).read()
    }

    pub fn update_bases(&mut self, gicd: usize, gicc: usize) {
        self.dist_block = MmioBlock::new(gicd);
        self.cpu_block = MmioBlock::new(gicc);
        GIC_INITIALIZED.store(false, Ordering::Relaxed);
        GIC_VERSION.store(detect_gic_version(gicd), Ordering::Relaxed);
    }

    pub fn gicd_base_addr(&self) -> usize {
        // This is a bit of a hack, but MmioBlock doesn't easily expose its base.
        // We'll return 0 for now as it's mostly used for version detection which we handle in update_bases.
        0
    }

    pub fn read_iar(&self) -> u32 {
        unsafe { self.read_cpu(bits::GICC_IAR) }
    }
}

impl InterruptController for Gic {
    unsafe fn init(&mut self) {
        // Enable GIC Distributor (GICD_CTLR)
        self.write_dist(bits::GICD_CTLR, 1);

        // Enable CPU Interface (GICC_CTLR)
        self.write_cpu(bits::GICC_CTLR, 1);

        // Priority mask configured via Cargo metadata (0-255).
        self.write_cpu(
            bits::GICC_PMR,
            (AARCH64_GIC_CPU_PRIORITY_MASK as u32).min(0xFF),
        );

        GIC_INITIALIZED.store(true, Ordering::Relaxed);
        // Version detection happens in update_bases.
    }

    unsafe fn enable_interrupt(&mut self, irq: u32) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ISENABLERn
        self.write_dist(bits::GICD_ISENABLER + reg as usize, 1 << bit);
    }

    unsafe fn disable_interrupt(&mut self, irq: u32) {
        let reg = (irq / 32) * 4;
        let bit = irq % 32;
        // GICD_ICENABLERn
        self.write_dist(bits::GICD_ICENABLER + reg as usize, 1 << bit);
    }

    unsafe fn end_of_interrupt(&mut self, irq: u32) {
        self.write_cpu(bits::GICC_EOIR, irq);
    }
}
