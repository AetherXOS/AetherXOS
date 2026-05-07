use core::sync::atomic::Ordering;
use crate::config::KernelConfig;
use super::queues::{build_create_io_cq_sqe, build_create_io_sq_sqe, cq_doorbell_offset, sq_doorbell_offset, CQE_DW3_CID_MASK, CQE_DW3_PHASE_BIT, CQE_DW3_SF_MASK, CQE_DW3_SF_SHIFT};
use super::*;

impl Nvme {
    // ── MMIO helpers ──────────────────────────────────────────────────────────

    pub(super) unsafe fn read32(&self, offset: u64) -> u32 {
        unsafe { core::ptr::read_volatile((self.mmio_base + offset) as *const u32) }
    }

    pub(super) unsafe fn write32(&self, offset: u64, val: u32) {
        unsafe { core::ptr::write_volatile((self.mmio_base + offset) as *mut u32, val) };
    }

    pub(super) unsafe fn read64(&self, offset: u64) -> u64 {
        unsafe { core::ptr::read_volatile((self.mmio_base + offset) as *const u64) }
    }

    pub(super) unsafe fn write64(&self, offset: u64, val: u64) {
        unsafe { core::ptr::write_volatile((self.mmio_base + offset) as *mut u64, val) };
    }

    // ── Controller init ───────────────────────────────────────────────────────

    pub(super) fn controller_init(&mut self) -> Result<(), &'static str> {
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

    pub(super) unsafe fn submit_and_poll_admin(&mut self, sqe: [u32; 16]) -> Result<(), &'static str> {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let sq_virt = (self.admin_sq_phys + hhdm) as usize;
        let cq_virt = (self.admin_cq_phys + hhdm) as usize;

        // Current slot
        let slot = self.admin_sq_tail as usize % 2; // Size is 2
        let sqe_ptr = (sq_virt + slot * 64) as *mut u32;
        for (i, &dw) in sqe.iter().enumerate() {
            unsafe { core::ptr::write_volatile(sqe_ptr.add(i), dw) };
        }

        self.admin_sq_tail = self.admin_sq_tail.wrapping_add(1);
        core::sync::atomic::fence(Ordering::SeqCst);
        unsafe {
            self.write32(
                sq_doorbell_offset(0, self.doorbell_stride),
                self.admin_sq_tail as u32,
            )
        };

        let expected_cid = (sqe[0] >> 16) as u16;
        let poll_slot = self.admin_cq_head as usize % 2;
        let cqe_dw3_ptr = (cq_virt + poll_slot * 16 + 12) as *const u32;

        for _ in 0..KernelConfig::nvme_poll_timeout_spins() {
            let dw3 = unsafe { core::ptr::read_volatile(cqe_dw3_ptr) };
            let phase = ((dw3 & CQE_DW3_PHASE_BIT) >> 16) as u8;
            let cid = (dw3 & CQE_DW3_CID_MASK) as u16;

            if phase == self.admin_cq_phase && cid == expected_cid {
                self.admin_cq_head = self.admin_cq_head.wrapping_add(1);
                if self.admin_cq_head % 2 == 0 {
                    self.admin_cq_phase ^= 1;
                }
                unsafe {
                    self.write32(
                        cq_doorbell_offset(0, self.doorbell_stride),
                        self.admin_cq_head as u32,
                    )
                };

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
}
