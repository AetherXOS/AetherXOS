use crate::config::KernelConfig;
use crate::hal::pci::{PciDevice, CLASS_MASS_STORAGE};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::block::{mark_init, mark_io, mark_probe, BlockDevice, BlockDeviceInfo, BlockDriverKind};
use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{pci_bar0_mmio_base, pci_class, probe_first_pci_by_class};
use crate::impl_lifecycle_adapter;

mod profile;
mod queues;

pub use profile::{
    nvme_effective_io_queue_depth, nvme_io_queue_depth_override, nvme_queue_profile,
    set_nvme_io_queue_depth_override, set_nvme_queue_profile, wait_stats, NvmeQueueProfile,
    NvmeWaitStats,
};
use queues::{
    build_create_io_cq_sqe, build_create_io_sq_sqe, build_io_sqe, cq_doorbell_offset,
    sq_doorbell_offset, CQE_DW3_CID_MASK, CQE_DW3_PHASE_BIT, CQE_DW3_SF_MASK, CQE_DW3_SF_SHIFT,
    DEFAULT_DOORBELL_STRIDE, NVME_CMD_READ, NVME_CMD_WRITE,
};

// ══════════════════════════════════════════════════════════════════════════════
// NVMe Controller Register Offsets (NVMe Base Spec 1.4 §3.1)
// ══════════════════════════════════════════════════════════════════════════════

/// Capabilities (64-bit)
const NVME_REG_CAP: u64 = 0x0000;
/// Controller Configuration
const NVME_REG_CC: u64 = 0x0014;
/// Controller Status
const NVME_REG_CSTS: u64 = 0x001C;
/// Admin Queue Attributes
const NVME_REG_AQA: u64 = 0x0024;
/// Admin Submission Queue Base Address (64-bit)
const NVME_REG_ASQ: u64 = 0x0028;
/// Admin Completion Queue Base Address (64-bit)
const NVME_REG_ACQ: u64 = 0x0030;

// CC register bit masks
const CC_EN: u32 = 1 << 0; // Enable
const CC_CSS_NVM: u32 = 0b000 << 4; // NVM Command Set
const CC_MPS_4K: u32 = 0 << 7; // Memory page size = 4 KiB
const CC_AMS_RR: u32 = 0 << 11; // Round-robin arbitration
const CC_SHN_NONE: u32 = 0 << 14; // No shutdown
const CC_IOSQES: u32 = 6 << 16; // SQ entry size = 2^6 = 64 bytes
const CC_IOCQES: u32 = 4 << 20; // CQ entry size = 2^4 = 16 bytes

// CSTS register bits
const CSTS_RDY: u32 = 1 << 0; // Controller ready
const CSTS_CFS: u32 = 1 << 1; // Fatal status

// ══════════════════════════════════════════════════════════════════════════════
// NVMe PCI Subclass
// ══════════════════════════════════════════════════════════════════════════════

/// PCI subclass 0x08 = Non-Volatile Memory controller
const PCI_SUBCLASS_NVME: u8 = 0x08;

static NVME_DISABLE_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static NVME_CONTROLLER_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static NVME_ADMIN_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static NVME_IO_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

// ══════════════════════════════════════════════════════════════════════════════
// Queue profiles
// ══════════════════════════════════════════════════════════════════════════════

// ══════════════════════════════════════════════════════════════════════════════
// NVMe controller struct
// ══════════════════════════════════════════════════════════════════════════════

pub struct Nvme {
    pub mmio_base: u64,
    pub irq: u8,
    pub io_queue_depth: usize,

    /// Doorbell stride in bytes (derived from CAP.DSTRD).
    doorbell_stride: u64,

    admin_sq_phys: u64,
    admin_cq_phys: u64,
    admin_sq_tail: u16,
    admin_cq_head: u16,
    admin_cq_phase: u8,

    io_sq_phys: u64,
    io_cq_phys: u64,
    io_sq_tail: u16,
    io_cq_head: u16,
    io_cq_phase: u8,

    /// Monotonic command identifier (wraps at u16::MAX).
    next_cid: u16,

    lifecycle: DriverStateMachine,
}

impl Nvme {
    pub fn probe(devices: &[PciDevice]) -> Option<Self> {
        let Some(dev) =
            probe_first_pci_by_class(devices, pci_class(CLASS_MASS_STORAGE, PCI_SUBCLASS_NVME))
        else {
            mark_probe(false);
            return None;
        };
        let Some(mmio_base) = pci_bar0_mmio_base(dev) else {
            mark_probe(false);
            return None;
        };
        mark_probe(true);
        Some(Self {
            mmio_base,
            irq: dev.interrupt_line,
            io_queue_depth: nvme_effective_io_queue_depth(),
            doorbell_stride: DEFAULT_DOORBELL_STRIDE,
            admin_sq_phys: 0,
            admin_cq_phys: 0,
            admin_sq_tail: 0,
            admin_cq_head: 0,
            admin_cq_phase: 1,
            io_sq_phys: 0,
            io_cq_phys: 0,
            io_sq_tail: 0,
            io_cq_head: 0,
            io_cq_phase: 1,
            next_cid: 1,
            lifecycle: DriverStateMachine::new_discovered(),
        })
    }

    // ── MMIO helpers ──────────────────────────────────────────────────────────

    unsafe fn read32(&self, offset: u64) -> u32 {
        core::ptr::read_volatile((self.mmio_base + offset) as *const u32)
    }

    unsafe fn write32(&self, offset: u64, val: u32) {
        core::ptr::write_volatile((self.mmio_base + offset) as *mut u32, val);
    }

    unsafe fn read64(&self, offset: u64) -> u64 {
        core::ptr::read_volatile((self.mmio_base + offset) as *const u64)
    }

    unsafe fn write64(&self, offset: u64, val: u64) {
        core::ptr::write_volatile((self.mmio_base + offset) as *mut u64, val);
    }

    // ── Controller init ───────────────────────────────────────────────────────

    fn controller_init(&mut self) -> Result<(), &'static str> {
        unsafe {
            // 1. Read CAP and extract DSTRD (bits 35:32).
            let cap = self.read64(NVME_REG_CAP);
            let dstrd_field = ((cap >> 32) & 0xF) as u64;
            self.doorbell_stride = 4 << dstrd_field;

            // 2. Disable controller.
            let mut cc = self.read32(NVME_REG_CC);
            cc &= !CC_EN;
            self.write32(NVME_REG_CC, cc);

            // Wait for CSTS.RDY = 0 (controller disabled).
            let mut disabled = false;
            for _ in 0..KernelConfig::nvme_disable_ready_timeout_spins() {
                if (self.read32(NVME_REG_CSTS) & CSTS_RDY) == 0 {
                    disabled = true;
                    break;
                }
                core::hint::spin_loop();
            }
            if !disabled {
                NVME_DISABLE_READY_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                return Err("nvme: controller disable timeout");
            }

            // 3. Allocate admin queues.
            static mut ADMIN_SQ: [u8; 4096] = [0u8; 4096];
            static mut ADMIN_CQ: [u8; 4096] = [0u8; 4096];

            let asq_virt = (&raw mut ADMIN_SQ) as usize;
            let acq_virt = (&raw mut ADMIN_CQ) as usize;

            let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
            let asq_phys = if asq_virt as u64 >= hhdm {
                asq_virt as u64 - hhdm
            } else {
                asq_virt as u64
            };
            let acq_phys = if acq_virt as u64 >= hhdm {
                acq_virt as u64 - hhdm
            } else {
                acq_virt as u64
            };

            self.admin_sq_phys = asq_phys;
            self.admin_cq_phys = acq_phys;

            // AQA: SQ size=2 (0-indexed => 1), CQ size=2 (0-indexed => 1)
            self.write32(NVME_REG_AQA, 0x01 | (0x01 << 16));
            self.write64(NVME_REG_ASQ, asq_phys);
            self.write64(NVME_REG_ACQ, acq_phys);

            // 4. Set CC and enable.
            // IOSQES=6(64B), IOCQES=4(16B)
            let new_cc =
                CC_CSS_NVM | CC_MPS_4K | CC_AMS_RR | CC_SHN_NONE | CC_IOSQES | CC_IOCQES | CC_EN;
            self.write32(NVME_REG_CC, new_cc);

            // Wait for CSTS.RDY = 1.
            let poll_timeout_spins = KernelConfig::nvme_poll_timeout_spins();
            for i in 0..poll_timeout_spins {
                let csts = self.read32(NVME_REG_CSTS);
                if (csts & CSTS_CFS) != 0 {
                    return Err("nvme: controller fatal status during init");
                }
                if (csts & CSTS_RDY) != 0 {
                    break;
                }
                if i == poll_timeout_spins - 1 {
                    NVME_CONTROLLER_READY_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                    return Err("nvme: controller ready timeout");
                }
                core::hint::spin_loop();
            }

            // 5. Create I/O Queues (Queue ID 1)
            static mut IO_SQ: [u8; 4096] = [0u8; 4096];
            static mut IO_CQ: [u8; 4096] = [0u8; 4096];
            let iosq_virt = (&raw mut IO_SQ) as usize;
            let iocq_virt = (&raw mut IO_CQ) as usize;
            let iosq_phys = if iosq_virt as u64 >= hhdm {
                iosq_virt as u64 - hhdm
            } else {
                iosq_virt as u64
            };
            let iocq_phys = if iocq_virt as u64 >= hhdm {
                iocq_virt as u64 - hhdm
            } else {
                iocq_virt as u64
            };

            self.io_sq_phys = iosq_phys;
            self.io_cq_phys = iocq_phys;

            // Create I/O Completion Queue
            let cid1 = self.next_cid();
            let sqe_cq = build_create_io_cq_sqe(cid1, 1, iocq_phys, 16);
            self.submit_and_poll_admin(sqe_cq)?;

            // Create I/O Submission Queue
            let cid2 = self.next_cid();
            let sqe_sq = build_create_io_sq_sqe(cid2, 1, 1, iosq_phys, 16);
            self.submit_and_poll_admin(sqe_sq)?;

            self.lifecycle.on_init_success();
        }
        Ok(())
    }

    // ── Command submission ────────────────────────────────────────────────────

    unsafe fn submit_and_poll_admin(&mut self, sqe: [u32; 16]) -> Result<(), &'static str> {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let sq_virt = (self.admin_sq_phys + hhdm) as usize;
        let cq_virt = (self.admin_cq_phys + hhdm) as usize;

        // Current slot
        let slot = self.admin_sq_tail as usize % 2; // Size is 2
        let sqe_ptr = (sq_virt + slot * 64) as *mut u32;
        for (i, &dw) in sqe.iter().enumerate() {
            core::ptr::write_volatile(sqe_ptr.add(i), dw);
        }

        self.admin_sq_tail = self.admin_sq_tail.wrapping_add(1);
        core::sync::atomic::fence(Ordering::SeqCst);
        self.write32(
            sq_doorbell_offset(0, self.doorbell_stride),
            self.admin_sq_tail as u32,
        );

        let expected_cid = (sqe[0] >> 16) as u16;
        let poll_slot = self.admin_cq_head as usize % 2;
        let cqe_dw3_ptr = (cq_virt + poll_slot * 16 + 12) as *const u32;

        for _ in 0..KernelConfig::nvme_poll_timeout_spins() {
            let dw3 = core::ptr::read_volatile(cqe_dw3_ptr);
            let phase = ((dw3 & CQE_DW3_PHASE_BIT) >> 16) as u8;
            let cid = (dw3 & CQE_DW3_CID_MASK) as u16;

            if phase == self.admin_cq_phase && cid == expected_cid {
                self.admin_cq_head = self.admin_cq_head.wrapping_add(1);
                if self.admin_cq_head % 2 == 0 {
                    self.admin_cq_phase ^= 1;
                }
                self.write32(
                    cq_doorbell_offset(0, self.doorbell_stride),
                    self.admin_cq_head as u32,
                );

                let sf = (dw3 >> CQE_DW3_SF_SHIFT) & CQE_DW3_SF_MASK;
                if sf != 0 {
                    return Err("nvme: admin command error");
                }
                return Ok(());
            }
            core::hint::spin_loop();
        }
        NVME_ADMIN_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        Err("nvme: admin timeout")
    }

    unsafe fn submit_and_poll_io(&mut self, sqe: [u32; 16]) -> Result<(), &'static str> {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let sq_virt = (self.io_sq_phys + hhdm) as usize;
        let cq_virt = (self.io_cq_phys + hhdm) as usize;

        let slot = self.io_sq_tail as usize % 16; // Use size 16 for IO
        let sqe_ptr = (sq_virt + slot * 64) as *mut u32;
        for (i, &dw) in sqe.iter().enumerate() {
            core::ptr::write_volatile(sqe_ptr.add(i), dw);
        }

        self.io_sq_tail = self.io_sq_tail.wrapping_add(1);
        core::sync::atomic::fence(Ordering::SeqCst);
        self.write32(
            sq_doorbell_offset(1, self.doorbell_stride),
            self.io_sq_tail as u32,
        );

        let expected_cid = (sqe[0] >> 16) as u16;

        for _ in 0..KernelConfig::nvme_io_timeout_spins() {
            let poll_slot = self.io_cq_head as usize % 16;
            let cqe_dw3_ptr = (cq_virt + poll_slot * 16 + 12) as *const u32;
            let dw3 = core::ptr::read_volatile(cqe_dw3_ptr);
            let phase = ((dw3 & CQE_DW3_PHASE_BIT) >> 16) as u8;
            let cid = (dw3 & CQE_DW3_CID_MASK) as u16;

            if phase == self.io_cq_phase && cid == expected_cid {
                self.io_cq_head = self.io_cq_head.wrapping_add(1);
                if self.io_cq_head % 16 == 0 {
                    self.io_cq_phase ^= 1;
                }
                self.write32(
                    cq_doorbell_offset(1, self.doorbell_stride),
                    self.io_cq_head as u32,
                );

                let sf = (dw3 >> CQE_DW3_SF_SHIFT) & CQE_DW3_SF_MASK;
                if sf != 0 {
                    return Err("nvme: io command error");
                }
                return Ok(());
            }
            core::hint::spin_loop();
        }
        NVME_IO_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        Err("nvme: io timeout")
    }

    fn next_cid(&mut self) -> u16 {
        let cid = self.next_cid;
        self.next_cid = self.next_cid.wrapping_add(1).max(1); // never 0
        cid
    }

    fn buf_to_phys(buf: *const u8) -> u64 {
        let virt = buf as u64;
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        if virt >= hhdm {
            virt - hhdm
        } else {
            virt
        }
    }

    // ── Lifecycle adapter implementations ─────────────────────────────────────

    fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();
        if self.mmio_base == 0 {
            self.lifecycle.on_init_failure(DriverErrorKind::Init);
            return Err("nvme mmio base invalid");
        }
        match self.controller_init() {
            Ok(()) => {
                self.io_queue_depth = nvme_effective_io_queue_depth();
                mark_init(true);
                self.lifecycle.on_init_success();
                Ok(())
            }
            Err(e) => {
                mark_init(false);
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                Err(e)
            }
        }
    }

    fn lifecycle_service(&mut self) -> Result<(), &'static str> {
        match self.lifecycle.io_gate() {
            DriverIoGate::Open => {}
            DriverIoGate::Cooldown => return Err("nvme recovery cooldown active"),
            DriverIoGate::Closed => return Err("nvme driver unhealthy"),
        }
        self.lifecycle.on_io_success();
        Ok(())
    }

    fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
        // Safely disable the controller before teardown.
        unsafe {
            let mut cc = self.read32(NVME_REG_CC);
            cc &= !CC_EN;
            self.write32(NVME_REG_CC, cc);
        }
        self.lifecycle.on_teardown();
        Ok(())
    }
}

impl PciProbeDriver for Nvme {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self> {
        Self::probe(devices)
    }
}

impl_lifecycle_adapter!(
    for Nvme,
    class: DriverClass::Storage,
    name: "nvme",
    lifecycle: lifecycle,
    init: lifecycle_init,
    service: lifecycle_service,
    teardown: lifecycle_teardown,
);

impl BlockDevice for Nvme {
    fn info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            kind: BlockDriverKind::Nvme,
            io_base: self.mmio_base,
            irq: self.irq,
            block_size: crate::modules::drivers::block::SECTOR_SIZE as u32,
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        self.lifecycle_init()
    }

    fn read_blocks(&mut self, lba: u64, count: u16, out: &mut [u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * crate::modules::drivers::block::SECTOR_SIZE;
        if out.len() < bytes {
            return Err("nvme: output buffer too small");
        }

        let prp1 = Self::buf_to_phys(out.as_ptr());
        let cid = self.next_cid();
        let sqe = build_io_sqe(NVME_CMD_READ, cid, 1, prp1, lba, count.saturating_sub(1));

        match unsafe { self.submit_and_poll_io(sqe) } {
            Ok(()) => {
                mark_io(true, 100_000);
                Ok(bytes)
            }
            Err(e) => {
                mark_io(false, 0);
                Err(e)
            }
        }
    }

    fn write_blocks(&mut self, lba: u64, count: u16, input: &[u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * crate::modules::drivers::block::SECTOR_SIZE;
        if input.len() < bytes {
            return Err("nvme: input buffer too small");
        }

        let prp1 = Self::buf_to_phys(input.as_ptr());
        let cid = self.next_cid();
        let sqe = build_io_sqe(NVME_CMD_WRITE, cid, 1, prp1, lba, count.saturating_sub(1));

        match unsafe { self.submit_and_poll_io(sqe) } {
            Ok(()) => {
                mark_io(true, 120_000);
                Ok(bytes)
            }
            Err(e) => {
                mark_io(false, 0);
                Err(e)
            }
        }
    }
}
