use crate::hal::pci::{PciDevice, CLASS_MASS_STORAGE};
use core::sync::atomic::{AtomicU64};

use super::block::mark_probe;
use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{pci_bar0_mmio_base, pci_class, probe_first_pci_by_class};
use crate::impl_lifecycle_adapter;

pub mod profile;
pub mod queues;
pub mod controller;
pub mod ops;

pub use profile::{
    nvme_effective_io_queue_depth, nvme_io_queue_depth_override, nvme_queue_profile,
    set_nvme_io_queue_depth_override, set_nvme_queue_profile, wait_stats, NvmeQueueProfile,
    NvmeWaitStats,
};

// ─── Constants ───────────────────────────────────────────────────────

pub(crate) const NVME_REG_CAP: u64 = 0x0000;
pub(crate) const NVME_REG_CC: u64 = 0x0014;
pub(crate) const NVME_REG_CSTS: u64 = 0x001C;
pub(crate) const NVME_REG_AQA: u64 = 0x0024;
pub(crate) const NVME_REG_ASQ: u64 = 0x0028;
pub(crate) const NVME_REG_ACQ: u64 = 0x0030;

pub(crate) const CC_EN: u32 = 1 << 0;
pub(crate) const CC_CSS_NVM: u32 = 0b000 << 4;
pub(crate) const CC_MPS_4K: u32 = 0 << 7;
pub(crate) const CC_AMS_RR: u32 = 0 << 11;
pub(crate) const CC_SHN_NONE: u32 = 0 << 14;
pub(crate) const CC_IOSQES: u32 = 6 << 16;
pub(crate) const CC_IOCQES: u32 = 4 << 20;

pub(crate) const CSTS_RDY: u32 = 1 << 0;
pub(crate) const CSTS_CFS: u32 = 1 << 1;

pub(crate) const PCI_SUBCLASS_NVME: u8 = 0x08;
pub(crate) const DEFAULT_DOORBELL_STRIDE: u64 = 4;

pub(crate) static NVME_DISABLE_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub(crate) static NVME_CONTROLLER_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub(crate) static NVME_ADMIN_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub(crate) static NVME_IO_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

// ─── Controller Struct ───────────────────────────────────────────────

pub struct Nvme {
    pub mmio_base: u64,
    pub irq: u8,
    pub io_queue_depth: usize,
    pub(crate) doorbell_stride: u64,

    pub(crate) admin_sq_phys: u64,
    pub(crate) admin_cq_phys: u64,
    pub(crate) admin_sq_tail: u16,
    pub(crate) admin_cq_head: u16,
    pub(crate) admin_cq_phase: u8,

    pub(crate) io_sq_phys: u64,
    pub(crate) io_cq_phys: u64,
    pub(crate) io_sq_tail: u16,
    pub(crate) io_cq_head: u16,
    pub(crate) io_cq_phase: u8,

    pub(crate) next_cid: u16,
    pub(crate) lifecycle: DriverStateMachine,
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

    pub(crate) fn next_cid(&mut self) -> u16 {
        let cid = self.next_cid;
        self.next_cid = self.next_cid.wrapping_add(1).max(1);
        cid
    }

    pub(crate) fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();
        if self.mmio_base == 0 {
            self.lifecycle.on_init_failure(DriverErrorKind::Init);
            return Err("nvme mmio base invalid");
        }
        match self.controller_init() {
            Ok(()) => {
                self.io_queue_depth = nvme_effective_io_queue_depth();
                super::block::mark_init(true);
                self.lifecycle.on_init_success();
                Ok(())
            }
            Err(e) => {
                super::block::mark_init(false);
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                Err(e)
            }
        }
    }

    pub(crate) fn lifecycle_service(&mut self) -> Result<(), &'static str> {
        match self.lifecycle.io_gate() {
            DriverIoGate::Open => {}
            DriverIoGate::Cooldown => return Err("nvme recovery cooldown active"),
            DriverIoGate::Closed => return Err("nvme driver unhealthy"),
        }
        self.lifecycle.on_io_success();
        Ok(())
    }

    pub(crate) fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
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
