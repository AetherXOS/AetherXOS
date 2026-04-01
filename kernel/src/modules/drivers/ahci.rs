use crate::config::KernelConfig;
use crate::hal::pci::{PciDevice, CLASS_MASS_STORAGE};
use core::sync::atomic::{AtomicU64, Ordering};

use super::block::{mark_init, mark_io, mark_probe, BlockDevice, BlockDeviceInfo, BlockDriverKind};
use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{pci_bar0_mmio_base, pci_class, probe_first_pci_by_class};
use crate::impl_lifecycle_adapter;

static AHCI_READ_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static AHCI_WRITE_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct AhciWaitStats {
    pub io_timeout_spins: usize,
    pub read_timeouts: u64,
    pub write_timeouts: u64,
}

pub fn wait_stats() -> AhciWaitStats {
    AhciWaitStats {
        io_timeout_spins: KernelConfig::ahci_io_timeout_spins(),
        read_timeouts: AHCI_READ_TIMEOUTS.load(Ordering::Relaxed),
        write_timeouts: AHCI_WRITE_TIMEOUTS.load(Ordering::Relaxed),
    }
}

pub struct Ahci {
    pub abar: u64,
    pub irq: u8,
    pub active_port: u32,
    lifecycle: DriverStateMachine,
}

impl Ahci {
    pub fn probe(devices: &[PciDevice]) -> Option<Self> {
        let Some(dev) = probe_first_pci_by_class(devices, pci_class(CLASS_MASS_STORAGE, 0x06))
        else {
            mark_probe(false);
            return None;
        };
        let Some(abar) = pci_bar0_mmio_base(dev) else {
            mark_probe(false);
            return None;
        };
        let out = Self {
            abar,
            irq: dev.interrupt_line,
            active_port: u32::MAX, // Initialised in init()
            lifecycle: DriverStateMachine::new_discovered(),
        };
        mark_probe(true);
        Some(out)
    }

    fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.init()
    }

    fn lifecycle_service(&mut self) -> Result<(), &'static str> {
        match self.lifecycle.io_gate() {
            DriverIoGate::Open => {}
            DriverIoGate::Cooldown => {
                return Err("ahci recovery cooldown active");
            }
            DriverIoGate::Closed => {
                return Err("ahci driver unhealthy");
            }
        }
        self.lifecycle.on_io_success();
        Ok(())
    }

    fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_teardown();
        Ok(())
    }

    fn port_base(&self) -> usize {
        self.abar as usize + 0x100 + (self.active_port as usize * 0x80)
    }
}

impl PciProbeDriver for Ahci {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self> {
        Self::probe(devices)
    }
}

impl_lifecycle_adapter!(
    for Ahci,
    class: DriverClass::Storage,
    name: "ahci",
    lifecycle: lifecycle,
    init: lifecycle_init,
    service: lifecycle_service,
    teardown: lifecycle_teardown,
);

impl BlockDevice for Ahci {
    fn info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            kind: BlockDriverKind::Ahci,
            io_base: self.abar,
            irq: self.irq,
            block_size: crate::modules::drivers::block::SECTOR_SIZE as u32,
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();
        if self.abar == 0 {
            self.lifecycle.on_init_failure(DriverErrorKind::Init);
            mark_init(false);
            return Err("ahci abar invalid");
        }

        // AHCI spec: PI (Ports Implemented) register is at ABAR + 0x0C
        let pi = unsafe { core::ptr::read_volatile((self.abar as usize + 0x0C) as *const u32) };
        if pi == 0 {
            self.lifecycle.on_init_failure(DriverErrorKind::Init);
            mark_init(false);
            return Err("ahci: no ports implemented");
        }

        // Find the first implemented port
        for i in 0..32 {
            if (pi & (1 << i)) != 0 {
                self.active_port = i;
                break;
            }
        }

        if self.active_port == u32::MAX {
            self.lifecycle.on_init_failure(DriverErrorKind::Init);
            mark_init(false);
            return Err("ahci: failed to find active port");
        }

        mark_init(true);
        self.lifecycle.on_init_success();
        Ok(())
    }

    fn read_blocks(&mut self, lba: u64, count: u16, out: &mut [u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * crate::modules::drivers::block::SECTOR_SIZE;
        if out.len() < bytes {
            mark_io(false, 0);
            self.lifecycle.on_io_failure(DriverErrorKind::Io);
            return Err("buffer too small");
        }
        if self.abar == 0 || self.active_port == u32::MAX {
            mark_io(false, 0);
            return Err("ahci controller offline");
        }

        let port_base = self.port_base();

        let phys_addr = match crate::hal::hhdm_offset() {
            Some(offset) => {
                let v = out.as_ptr() as usize;
                if v >= offset as usize {
                    (v - offset as usize) as u64
                } else {
                    v as u64
                }
            }
            None => return Err("ahci: no hhdm offset"),
        };

        unsafe {
            let pxclb_lo = core::ptr::read_volatile((port_base + 0x00) as *const u32) as u64;
            let pxclb_hi = core::ptr::read_volatile((port_base + 0x04) as *const u32) as u64;
            let cmd_list_phys = pxclb_lo | (pxclb_hi << 32);
            if cmd_list_phys == 0 {
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci: PxCLB not initialised");
            }

            let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
            let cmd_list_virt = (cmd_list_phys + hhdm) as *mut u32;

            core::ptr::write_volatile(cmd_list_virt.add(0), (1u32 << 16) | 5u32);
            core::ptr::write_volatile(cmd_list_virt.add(1), 0);

            let ctba_lo = core::ptr::read_volatile(cmd_list_virt.add(2)) as u64;
            let ctba_hi = core::ptr::read_volatile(cmd_list_virt.add(3)) as u64;
            let ctba_phys = ctba_lo | (ctba_hi << 32);
            if ctba_phys == 0 {
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci: command table not initialised");
            }
            let ct = (ctba_phys + hhdm) as *mut u8;

            ct.add(0).write_volatile(0x27);
            ct.add(1).write_volatile(0x80);
            ct.add(2).write_volatile(0x25); // READ DMA EXT
            ct.add(3).write_volatile(0x00);
            ct.add(4).write_volatile((lba & 0xFF) as u8);
            ct.add(5).write_volatile(((lba >> 8) & 0xFF) as u8);
            ct.add(6).write_volatile(((lba >> 16) & 0xFF) as u8);
            ct.add(7).write_volatile(0x40); // LBA mode
            ct.add(8).write_volatile(((lba >> 24) & 0xFF) as u8);
            ct.add(9).write_volatile(((lba >> 32) & 0xFF) as u8);
            ct.add(10).write_volatile(((lba >> 40) & 0xFF) as u8);
            ct.add(11).write_volatile(0x00);
            ct.add(12).write_volatile((count & 0xFF) as u8);
            ct.add(13).write_volatile(((count >> 8) & 0xFF) as u8);

            let prdt = ct.add(0x80) as *mut u32;
            core::ptr::write_volatile(prdt.add(0), (phys_addr & 0xFFFF_FFFF) as u32);
            core::ptr::write_volatile(prdt.add(1), (phys_addr >> 32) as u32);
            core::ptr::write_volatile(prdt.add(2), 0);
            core::ptr::write_volatile(prdt.add(3), (bytes as u32 - 1) | (1 << 31));

            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

            let pxis_val = core::ptr::read_volatile((port_base + 0x10) as *const u32);
            core::ptr::write_volatile((port_base + 0x10) as *mut u32, pxis_val);

            core::ptr::write_volatile((port_base + 0x38) as *mut u32, 1u32);

            let mut done = false;
            for _ in 0..KernelConfig::ahci_io_timeout_spins() {
                let pxtfd = core::ptr::read_volatile((port_base + 0x20) as *const u32);
                if (pxtfd & 0x88) == 0 {
                    let pxci = core::ptr::read_volatile((port_base + 0x38) as *const u32);
                    if (pxci & 1) == 0 {
                        done = true;
                        break;
                    }
                }
                if (pxtfd & 0x01) != 0 {
                    mark_io(false, 0);
                    self.lifecycle.on_io_failure(DriverErrorKind::Io);
                    return Err("ahci read: device error");
                }
                core::hint::spin_loop();
            }

            if !done {
                AHCI_READ_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci read: timeout");
            }
        }

        mark_io(true, 100_000);
        self.lifecycle.on_io_success();
        Ok(bytes)
    }

    fn write_blocks(&mut self, lba: u64, count: u16, input: &[u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * crate::modules::drivers::block::SECTOR_SIZE;
        if input.len() < bytes {
            mark_io(false, 0);
            self.lifecycle.on_io_failure(DriverErrorKind::Io);
            return Err("input too small");
        }
        if self.abar == 0 || self.active_port == u32::MAX {
            mark_io(false, 0);
            return Err("ahci controller offline");
        }

        let port_base = self.port_base();

        let phys_addr = match crate::hal::hhdm_offset() {
            Some(offset) => {
                let v = input.as_ptr() as usize;
                if v >= offset as usize {
                    (v - offset as usize) as u64
                } else {
                    v as u64
                }
            }
            None => return Err("ahci: no hhdm offset"),
        };

        unsafe {
            let pxclb_lo = core::ptr::read_volatile((port_base + 0x00) as *const u32) as u64;
            let pxclb_hi = core::ptr::read_volatile((port_base + 0x04) as *const u32) as u64;
            let cmd_list_phys = pxclb_lo | (pxclb_hi << 32);
            if cmd_list_phys == 0 {
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci: PxCLB not initialised");
            }

            let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
            let cmd_list_virt = (cmd_list_phys + hhdm) as *mut u32;

            core::ptr::write_volatile(cmd_list_virt.add(0), (1u32 << 16) | 5u32 | (1 << 6)); // W=1
            core::ptr::write_volatile(cmd_list_virt.add(1), 0);

            let ctba_lo = core::ptr::read_volatile(cmd_list_virt.add(2)) as u64;
            let ctba_hi = core::ptr::read_volatile(cmd_list_virt.add(3)) as u64;
            let ctba_phys = ctba_lo | (ctba_hi << 32);
            if ctba_phys == 0 {
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci: command table not initialised");
            }
            let ct = (ctba_phys + hhdm) as *mut u8;

            ct.add(0).write_volatile(0x27);
            ct.add(1).write_volatile(0x80);
            ct.add(2).write_volatile(0x35); // WRITE DMA EXT
            ct.add(3).write_volatile(0x00);
            ct.add(4).write_volatile((lba & 0xFF) as u8);
            ct.add(5).write_volatile(((lba >> 8) & 0xFF) as u8);
            ct.add(6).write_volatile(((lba >> 16) & 0xFF) as u8);
            ct.add(7).write_volatile(0x40); // LBA mode
            ct.add(8).write_volatile(((lba >> 24) & 0xFF) as u8);
            ct.add(9).write_volatile(((lba >> 32) & 0xFF) as u8);
            ct.add(10).write_volatile(((lba >> 40) & 0xFF) as u8);
            ct.add(11).write_volatile(0x00);
            ct.add(12).write_volatile((count & 0xFF) as u8);
            ct.add(13).write_volatile(((count >> 8) & 0xFF) as u8);

            let prdt = ct.add(0x80) as *mut u32;
            core::ptr::write_volatile(prdt.add(0), (phys_addr & 0xFFFF_FFFF) as u32);
            core::ptr::write_volatile(prdt.add(1), (phys_addr >> 32) as u32);
            core::ptr::write_volatile(prdt.add(2), 0);
            core::ptr::write_volatile(prdt.add(3), (bytes as u32 - 1) | (1 << 31));

            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

            let pxis_val = core::ptr::read_volatile((port_base + 0x10) as *const u32);
            core::ptr::write_volatile((port_base + 0x10) as *mut u32, pxis_val);

            core::ptr::write_volatile((port_base + 0x38) as *mut u32, 1u32);

            let mut done = false;
            for _ in 0..KernelConfig::ahci_io_timeout_spins() {
                let pxtfd = core::ptr::read_volatile((port_base + 0x20) as *const u32);
                if (pxtfd & 0x88) == 0 {
                    let pxci = core::ptr::read_volatile((port_base + 0x38) as *const u32);
                    if (pxci & 1) == 0 {
                        done = true;
                        break;
                    }
                }
                if (pxtfd & 0x01) != 0 {
                    mark_io(false, 0);
                    self.lifecycle.on_io_failure(DriverErrorKind::Io);
                    return Err("ahci write: device error");
                }
                core::hint::spin_loop();
            }

            if !done {
                AHCI_WRITE_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                mark_io(false, 0);
                self.lifecycle.on_io_failure(DriverErrorKind::Io);
                return Err("ahci write: timeout");
            }
        }

        mark_io(true, 100_000);
        self.lifecycle.on_io_success();
        Ok(bytes)
    }
}
