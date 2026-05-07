use core::sync::atomic::Ordering;
use crate::config::KernelConfig;
use super::queues::{build_io_sqe, cq_doorbell_offset, sq_doorbell_offset, CQE_DW3_CID_MASK, CQE_DW3_PHASE_BIT, CQE_DW3_SF_MASK, CQE_DW3_SF_SHIFT, NVME_CMD_READ, NVME_CMD_WRITE};
use super::super::block::{mark_io, BlockDevice, BlockDeviceInfo, BlockDriverKind};
use super::*;

impl Nvme {
    pub(super) unsafe fn submit_and_poll_io(&mut self, sqe: [u32; 16]) -> Result<(), &'static str> {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let sq_virt = (self.io_sq_phys + hhdm) as usize;
        let cq_virt = (self.io_cq_phys + hhdm) as usize;

        let slot = self.io_sq_tail as usize % 16; // Use size 16 for IO
        let sqe_ptr = (sq_virt + slot * 64) as *mut u32;
        for (i, &dw) in sqe.iter().enumerate() {
            unsafe { core::ptr::write_volatile(sqe_ptr.add(i), dw) };
        }

        self.io_sq_tail = self.io_sq_tail.wrapping_add(1);
        core::sync::atomic::fence(Ordering::SeqCst);
        unsafe {
            self.write32(
                sq_doorbell_offset(1, self.doorbell_stride),
                self.io_sq_tail as u32,
            )
        };

        let expected_cid = (sqe[0] >> 16) as u16;

        for _ in 0..KernelConfig::nvme_io_timeout_spins() {
            let poll_slot = self.io_cq_head as usize % 16;
            let cqe_dw3_ptr = (cq_virt + poll_slot * 16 + 12) as *const u32;
            let dw3 = unsafe { core::ptr::read_volatile(cqe_dw3_ptr) };
            let phase = ((dw3 & CQE_DW3_PHASE_BIT) >> 16) as u8;
            let cid = (dw3 & CQE_DW3_CID_MASK) as u16;

            if phase == self.io_cq_phase && cid == expected_cid {
                self.io_cq_head = self.io_cq_head.wrapping_add(1);
                if self.io_cq_head % 16 == 0 {
                    self.io_cq_phase ^= 1;
                }
                unsafe {
                    self.write32(
                        cq_doorbell_offset(1, self.doorbell_stride),
                        self.io_cq_head as u32,
                    )
                };

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

    pub(super) fn buf_to_phys(buf: *const u8) -> u64 {
        let virt = buf as u64;
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        if virt >= hhdm {
            virt - hhdm
        } else {
            virt
        }
    }
}

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
