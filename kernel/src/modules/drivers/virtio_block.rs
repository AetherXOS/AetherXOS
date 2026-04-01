use super::block::{mark_init, mark_io, mark_probe, BlockDevice, BlockDeviceInfo, BlockDriverKind};
use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{pci_bar0_io_base, pci_bar0_mmio_base};
use crate::hal::pci::PciDevice;
use crate::impl_lifecycle_adapter;
#[path = "virtio_block/queue.rs"]
mod queue;

use queue::{VirtQueue, VirtioBlkReq, VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT, VIRTQ_QUEUE_SIZE};

// ─── VirtIO Legacy PCI register offsets (I/O port space) ────────────────────
const VIRTIO_PCI_HOST_FEATURES: u16 = 0x00; // R
const VIRTIO_PCI_GUEST_FEATURES: u16 = 0x04; // W
const VIRTIO_PCI_QUEUE_PFN: u16 = 0x08; // W
const VIRTIO_PCI_QUEUE_SIZE: u16 = 0x0C; // R
const VIRTIO_PCI_QUEUE_SELECT: u16 = 0x0E; // W
const VIRTIO_PCI_QUEUE_NOTIFY: u16 = 0x10; // W
const VIRTIO_PCI_STATUS: u16 = 0x12; // RW
#[allow(dead_code)]
const VIRTIO_PCI_ISR: u16 = 0x13; // R

const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_FEATURE_BLK_SIZE: u32 = 1 << 6;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

pub struct VirtIoBlock {
    io_base: u32,
    mmio_base: u64,
    irq: u8,
    is_mmio: bool,
    lifecycle: DriverStateMachine,
    queue: Option<VirtQueue>,
    nsid: u32,
}

impl VirtIoBlock {
    pub fn probe(devices: &[PciDevice]) -> Option<Self> {
        // VirtIO PCI IDs are 0x1AF4 (Vendor) and 0x1001 (Block Legacy) or 0x1042 (Modern)
        let dev = devices
            .iter()
            .find(|d| d.vendor_id == 0x1AF4 && (d.device_id == 0x1001 || d.device_id == 0x1042))?;

        let (io_base, mmio_base, is_mmio) = if let Some(mmio) = pci_bar0_mmio_base(*dev) {
            (0, mmio, true)
        } else if let Some(io) = pci_bar0_io_base(*dev) {
            (u32::from(io), 0, false)
        } else {
            return None;
        };

        mark_probe(true);
        Some(Self {
            io_base,
            mmio_base,
            irq: dev.interrupt_line,
            is_mmio,
            lifecycle: DriverStateMachine::new_discovered(),
            queue: None,
            nsid: 1,
        })
    }

    // ── Legacy I/O port helpers ──────────────────────────────────────────────
    #[cfg(target_arch = "x86_64")]
    fn read8(&self, offset: u16) -> u8 {
        unsafe {
            x86_64::instructions::port::PortReadOnly::<u8>::new(self.io_base as u16 + offset).read()
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn read8(&self, _offset: u16) -> u8 {
        0
    }

    #[cfg(target_arch = "x86_64")]
    fn read16(&self, offset: u16) -> u16 {
        unsafe {
            x86_64::instructions::port::PortReadOnly::<u16>::new(self.io_base as u16 + offset)
                .read()
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn read16(&self, _offset: u16) -> u16 {
        0
    }

    #[cfg(target_arch = "x86_64")]
    fn read32(&self, offset: u16) -> u32 {
        unsafe {
            x86_64::instructions::port::PortReadOnly::<u32>::new(self.io_base as u16 + offset)
                .read()
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn read32(&self, _offset: u16) -> u32 {
        0
    }

    #[cfg(target_arch = "x86_64")]
    fn write8(&self, offset: u16, val: u8) {
        unsafe {
            x86_64::instructions::port::Port::<u8>::new(self.io_base as u16 + offset).write(val)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn write8(&self, _offset: u16, _val: u8) {}

    #[cfg(target_arch = "x86_64")]
    fn write16(&self, offset: u16, val: u16) {
        unsafe {
            x86_64::instructions::port::Port::<u16>::new(self.io_base as u16 + offset).write(val)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn write16(&self, _offset: u16, _val: u16) {}

    #[cfg(target_arch = "x86_64")]
    fn write32(&self, offset: u16, val: u32) {
        unsafe {
            x86_64::instructions::port::Port::<u32>::new(self.io_base as u16 + offset).write(val)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    fn write32(&self, _offset: u16, _val: u32) {}

    fn init_controller(&mut self) -> Result<(), &'static str> {
        if self.is_mmio {
            // MMIO-based VirtIO (modern): minimal skeleton init
            // Feature negotiation would happen via capability structures
            self.queue = Some(VirtQueue::new());
            return Ok(());
        }

        // Legacy I/O port based VirtIO block init sequence (VirtIO 0.9)
        // 1. Reset device
        self.write8(VIRTIO_PCI_STATUS, 0);
        // 2. Ack
        self.write8(VIRTIO_PCI_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
        // 3. Say we know how to drive it
        self.write8(
            VIRTIO_PCI_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER,
        );

        // 4. Feature negotiation — accept BLK_SIZE if offered
        let host_feats = self.read32(VIRTIO_PCI_HOST_FEATURES);
        let guest_feats = host_feats & VIRTIO_FEATURE_BLK_SIZE;
        self.write32(VIRTIO_PCI_GUEST_FEATURES, guest_feats);

        // 5. Set up queue 0 (the request queue)
        self.write16(VIRTIO_PCI_QUEUE_SELECT, 0);
        let qsz = self.read16(VIRTIO_PCI_QUEUE_SIZE) as usize;
        if qsz == 0 {
            return Err("virtio-block: queue size 0");
        }

        let mut q = VirtQueue::new();

        // Compute the page-frame number: legacy virtio expects the queue page-aligned
        // and provided as a PFN (physical address >> 12)
        let desc_phys = VirtQueue::phys(&q.desc);
        let pfn = (desc_phys >> 12) as u32;
        self.write32(VIRTIO_PCI_QUEUE_PFN, pfn);

        self.queue = Some(q);

        // 6. Driver OK
        self.write8(
            VIRTIO_PCI_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_DRIVER_OK,
        );

        Ok(())
    }

    /// Submit a single read or write request via the virtqueue and poll for completion.
    ///
    /// Layout: [header (device-ro)] [data (device-ro for write / device-rw for read)] [status (device-rw)]
    fn submit_io(&mut self, is_write: bool, lba: u64, data: &mut [u8]) -> Result<(), &'static str> {
        let q = self.queue.as_mut().ok_or("virtio-block: queue not ready")?;

        // Build the request header inline
        let req_type = if is_write {
            VIRTIO_BLK_T_OUT
        } else {
            VIRTIO_BLK_T_IN
        };
        let hdr = VirtioBlkReq {
            type_: req_type,
            reserved: 0,
            sector: lba,
        };
        let hdr_bytes: [u8; 16] = unsafe { core::mem::transmute(hdr) };

        // Allocate 3 descriptors: header, data, status byte
        let d0 = q.alloc_desc().ok_or("virtio-block: no free descriptors")?;
        let d1 = q.alloc_desc().ok_or("virtio-block: no free descriptors")?;
        let d2 = q.alloc_desc().ok_or("virtio-block: no free descriptors")?;

        let mut status: u8 = 0xFF;

        // d0: header (read-only for device)
        q.desc_write(
            d0 as usize,
            hdr_bytes.as_ptr() as u64,
            16,
            VIRTQ_DESC_F_NEXT,
            d1,
        );
        // d1: data buffer (write if reading from device, read-only if writing)
        let data_flags = if is_write {
            VIRTQ_DESC_F_NEXT
        } else {
            VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT
        };
        q.desc_write(
            d1 as usize,
            data.as_ptr() as u64,
            data.len() as u32,
            data_flags,
            d2,
        );
        // d2: status byte (always device-writeable)
        q.desc_write(
            d2 as usize,
            &status as *const u8 as u64,
            1,
            VIRTQ_DESC_F_WRITE,
            0,
        );

        q.avail_push(d0);

        // Notify device (queue index 0)
        if !self.is_mmio {
            self.write16(VIRTIO_PCI_QUEUE_NOTIFY, 0);
        }

        // Poll used ring until the device returns our descriptor
        const IO_POLL_TIMEOUT: u64 = 10_000_000;
        let mut spins: u64 = 0;
        let q = self.queue.as_mut().unwrap();
        loop {
            core::hint::spin_loop();
            let used_idx = q.used_idx();
            if used_idx != q.last_used {
                let slot = (q.last_used as usize % VIRTQ_QUEUE_SIZE) * 8 + 4;
                let _id = u32::from_le_bytes([
                    q.used[slot],
                    q.used[slot + 1],
                    q.used[slot + 2],
                    q.used[slot + 3],
                ]);
                q.last_used = q.last_used.wrapping_add(1);
                q.free_desc(d2);
                q.free_desc(d1);
                q.free_desc(d0);
                // status == 0 → success, 1 → failure, 2 → unsupported
                if status == 0 {
                    return Ok(());
                }
                return Err("virtio-block: device reported I/O error");
            }
            spins += 1;
            if spins > IO_POLL_TIMEOUT {
                break;
            }
        }

        // Timeout cleanup
        let q = self.queue.as_mut().unwrap();
        q.free_desc(d2);
        q.free_desc(d1);
        q.free_desc(d0);
        Err("virtio-block: I/O timeout")
    }
}

impl PciProbeDriver for VirtIoBlock {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self> {
        Self::probe(devices)
    }
}

impl_lifecycle_adapter!(
    for VirtIoBlock,
    class: DriverClass::Storage,
    name: "virtio-block",
    lifecycle: lifecycle,
    init: lifecycle_init,
    service: lifecycle_service,
    teardown: lifecycle_teardown,
);

impl VirtIoBlock {
    fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();
        match self.init_controller() {
            Ok(()) => {
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
            DriverIoGate::Open => Ok(()),
            _ => Err("virtio-block unhealthy"),
        }
    }

    fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_teardown();
        Ok(())
    }
}

impl BlockDevice for VirtIoBlock {
    fn info(&self) -> BlockDeviceInfo {
        BlockDeviceInfo {
            kind: BlockDriverKind::VirtIoBlock,
            io_base: if self.is_mmio {
                self.mmio_base
            } else {
                self.io_base as u64
            },
            irq: self.irq,
            block_size: 512,
        }
    }

    fn init(&mut self) -> Result<(), &'static str> {
        self.lifecycle_init()
    }

    fn read_blocks(&mut self, lba: u64, count: u16, out: &mut [u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * 512;
        if out.len() < bytes {
            return Err("virtio-block: output buffer too small");
        }
        let slice = &mut out[..bytes];
        match self.submit_io(false, lba, slice) {
            Ok(()) => {
                mark_io(true, 10_000);
                Ok(bytes)
            }
            Err(e) => {
                mark_io(false, 0);
                Err(e)
            }
        }
    }

    fn write_blocks(&mut self, lba: u64, count: u16, input: &[u8]) -> Result<usize, &'static str> {
        let bytes = usize::from(count) * 512;
        if input.len() < bytes {
            return Err("virtio-block: input buffer too small");
        }
        // write_blocks takes &[u8]; copy to a mutable buffer for the DMA path
        let mut buf = alloc::vec::Vec::from(&input[..bytes]);
        match self.submit_io(true, lba, &mut buf) {
            Ok(()) => {
                mark_io(true, 12_000);
                Ok(bytes)
            }
            Err(e) => {
                mark_io(false, 0);
                Err(e)
            }
        }
    }
}
