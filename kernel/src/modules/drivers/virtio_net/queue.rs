use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp;
use core::mem::size_of;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};

use super::regs::{
    VIRTIO_CTRL_MAX_CMD_BYTES, VIRTIO_NET_HDR_BYTES, VIRTIO_QUEUE_ALIGN, VIRTIO_QUEUE_MAX_SIZE,
    VIRTIO_QUEUE_MEMORY_BYTES, VIRTIO_RX_BUFFER_BYTES, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE,
};

const VIRTIO_CTRL_ACK_PENDING: u8 = 0xFF;

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C, align(4096))]
struct AlignedVirtQueueMemory([u8; VIRTIO_QUEUE_MEMORY_BYTES]);

#[derive(Clone, Copy)]
struct VirtQueueLayout {
    desc_off: usize,
    avail_off: usize,
    used_off: usize,
}

impl VirtQueueLayout {
    fn new(queue_size: usize) -> Result<Self, &'static str> {
        let desc_bytes = queue_size
            .checked_mul(size_of::<VirtqDesc>())
            .ok_or("virtio queue descriptor size overflow")?;
        let avail_ring_bytes = queue_size
            .checked_mul(size_of::<u16>())
            .ok_or("virtio queue avail ring size overflow")?;
        let avail_bytes = 6usize
            .checked_add(avail_ring_bytes)
            .ok_or("virtio queue avail size overflow")?;
        let used_bytes = 6usize
            .checked_add(
                queue_size
                    .checked_mul(size_of::<VirtqUsedElem>())
                    .ok_or("virtio queue used ring size overflow")?,
            )
            .ok_or("virtio queue used size overflow")?;

        let desc_off = 0usize;
        let avail_off = desc_off
            .checked_add(desc_bytes)
            .ok_or("virtio queue layout overflow")?;
        let used_off = align_up(
            avail_off
                .checked_add(avail_bytes)
                .ok_or("virtio queue layout overflow")?,
            VIRTIO_QUEUE_ALIGN,
        );
        if used_off
            .checked_add(used_bytes)
            .ok_or("virtio queue layout overflow")?
            > VIRTIO_QUEUE_MEMORY_BYTES
        {
            return Err("virtio queue memory insufficient");
        }

        Ok(Self {
            desc_off,
            avail_off,
            used_off,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum VirtQueueRole {
    Rx,
    Tx,
}

pub(super) struct VirtQueue {
    queue_index: u16,
    queue_size: u16,
    hhdm: u64,
    role: VirtQueueRole,
    memory: Box<AlignedVirtQueueMemory>,
    layout: VirtQueueLayout,
    next_avail_idx: u16,
    last_used_idx: u16,
    free_desc: Vec<u16>,
    buffers: Vec<Option<Vec<u8>>>,
}

impl VirtQueue {
    pub(super) fn new(
        queue_index: u16,
        queue_size: u16,
        role: VirtQueueRole,
        hhdm: u64,
    ) -> Result<Self, &'static str> {
        if queue_size == 0 {
            return Err("virtio queue size is zero");
        }
        let queue_size = cmp::min(queue_size, VIRTIO_QUEUE_MAX_SIZE);
        let layout = VirtQueueLayout::new(queue_size as usize)?;

        let mut out = Self {
            queue_index,
            queue_size,
            hhdm,
            role,
            memory: Box::new(AlignedVirtQueueMemory([0u8; VIRTIO_QUEUE_MEMORY_BYTES])),
            layout,
            next_avail_idx: 0,
            last_used_idx: 0,
            free_desc: Vec::new(),
            buffers: vec![None; queue_size as usize],
        };

        match role {
            VirtQueueRole::Tx => {
                for i in (0..queue_size).rev() {
                    out.free_desc.push(i);
                }
            }
            VirtQueueRole::Rx => {
                for i in 0..queue_size {
                    out.rearm_rx_descriptor(i)?;
                    out.publish_descriptor(i);
                }
            }
        }

        Ok(out)
    }

    pub(super) fn queue_index(&self) -> u16 {
        self.queue_index
    }

    pub(super) fn queue_size(&self) -> u16 {
        self.queue_size
    }

    pub(super) fn queue_phys_addr(&self) -> Result<u64, &'static str> {
        let virt = self.memory.0.as_ptr() as u64;
        if virt < self.hhdm {
            return Err("virtio queue address below hhdm");
        }
        Ok(virt - self.hhdm)
    }

    pub(super) fn submit_tx_frame(&mut self, frame: &[u8]) -> Result<(), &'static str> {
        if self.role != VirtQueueRole::Tx {
            return Err("virtio queue role mismatch");
        }

        let Some(desc_id) = self.free_desc.pop() else {
            return Err("virtio tx queue full");
        };

        let mut tx_packet = vec![0u8; VIRTIO_NET_HDR_BYTES + frame.len()];
        tx_packet[VIRTIO_NET_HDR_BYTES..].copy_from_slice(frame);
        let addr = self.buffer_phys_addr(&tx_packet)?;
        self.write_descriptor(desc_id, addr, tx_packet.len() as u32, 0, 0);
        self.buffers[desc_id as usize] = Some(tx_packet);
        self.publish_descriptor(desc_id);
        Ok(())
    }

    pub(super) fn poll_tx_completions(&mut self, budget: usize) -> usize {
        if self.role != VirtQueueRole::Tx {
            return 0;
        }

        let mut completed = 0usize;
        while completed < budget {
            let used_idx = self.used_index();
            if self.last_used_idx == used_idx {
                break;
            }

            let used_slot = (self.last_used_idx % self.queue_size) as usize;
            let elem = self.used_elem(used_slot);
            let desc_id = elem.id as u16;
            if desc_id < self.queue_size {
                self.buffers[desc_id as usize] = None;
                self.free_desc.push(desc_id);
            }

            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            completed += 1;
        }

        completed
    }

    pub(super) fn poll_rx_frames(
        &mut self,
        budget: usize,
        out_frames: &mut Vec<Vec<u8>>,
    ) -> (usize, usize) {
        if self.role != VirtQueueRole::Rx {
            return (0, 0);
        }

        let mut frames = 0usize;
        let mut rearmed = 0usize;
        while frames < budget {
            let used_idx = self.used_index();
            if self.last_used_idx == used_idx {
                break;
            }

            let used_slot = (self.last_used_idx % self.queue_size) as usize;
            let elem = self.used_elem(used_slot);
            let desc_id = elem.id as u16;
            if desc_id < self.queue_size {
                if let Some(buf) = self.buffers[desc_id as usize].as_ref() {
                    let used_len = cmp::min(elem.len as usize, buf.len());
                    if used_len > VIRTIO_NET_HDR_BYTES {
                        out_frames.push(buf[VIRTIO_NET_HDR_BYTES..used_len].to_vec());
                        frames += 1;
                    }
                }
                if self.rearm_rx_descriptor(desc_id).is_ok() {
                    self.publish_descriptor(desc_id);
                    rearmed += 1;
                }
            }

            self.last_used_idx = self.last_used_idx.wrapping_add(1);
        }

        (frames, rearmed)
    }

    fn rearm_rx_descriptor(&mut self, desc_id: u16) -> Result<(), &'static str> {
        if self.role != VirtQueueRole::Rx {
            return Err("virtio queue role mismatch");
        }
        if desc_id >= self.queue_size {
            return Err("virtio descriptor index out of range");
        }

        if self.buffers[desc_id as usize].is_none() {
            self.buffers[desc_id as usize] = Some(vec![0u8; VIRTIO_RX_BUFFER_BYTES]);
        }

        let Some(buf) = self.buffers[desc_id as usize].as_ref() else {
            return Err("virtio rx buffer unavailable");
        };

        let addr = self.buffer_phys_addr(buf)?;
        self.write_descriptor(desc_id, addr, buf.len() as u32, VIRTQ_DESC_F_WRITE, 0);
        Ok(())
    }

    fn buffer_phys_addr(&self, buf: &[u8]) -> Result<u64, &'static str> {
        let virt = buf.as_ptr() as u64;
        if virt < self.hhdm {
            return Err("virtio buffer address below hhdm");
        }
        Ok(virt - self.hhdm)
    }

    fn publish_descriptor(&mut self, desc_id: u16) {
        let slot = (self.next_avail_idx % self.queue_size) as usize;
        unsafe {
            let ring_ptr = self
                .memory
                .0
                .as_mut_ptr()
                .add(self.layout.avail_off + 4 + slot * size_of::<u16>())
                as *mut u16;
            write_volatile(ring_ptr, desc_id);
            compiler_fence(Ordering::Release);
            self.next_avail_idx = self.next_avail_idx.wrapping_add(1);
            let idx_ptr = self.memory.0.as_mut_ptr().add(self.layout.avail_off + 2) as *mut u16;
            write_volatile(idx_ptr, self.next_avail_idx);
        }
    }

    fn used_index(&self) -> u16 {
        unsafe {
            let ptr = self.memory.0.as_ptr().add(self.layout.used_off + 2) as *const u16;
            read_volatile(ptr)
        }
    }

    fn used_elem(&self, slot: usize) -> VirtqUsedElem {
        unsafe {
            let ptr = self
                .memory
                .0
                .as_ptr()
                .add(self.layout.used_off + 4 + slot * size_of::<VirtqUsedElem>())
                as *const VirtqUsedElem;
            read_volatile(ptr)
        }
    }

    fn write_descriptor(&mut self, desc_id: u16, addr: u64, len: u32, flags: u16, next: u16) {
        unsafe {
            let ptr = self
                .memory
                .0
                .as_mut_ptr()
                .add(self.layout.desc_off + desc_id as usize * size_of::<VirtqDesc>())
                as *mut VirtqDesc;
            write_volatile(
                ptr,
                VirtqDesc {
                    addr,
                    len,
                    flags,
                    next,
                },
            );
        }
    }
}

pub(super) struct VirtControlQueue {
    queue_index: u16,
    queue_size: u16,
    hhdm: u64,
    memory: Box<AlignedVirtQueueMemory>,
    layout: VirtQueueLayout,
    next_avail_idx: u16,
    last_used_idx: u16,
    inflight: bool,
    command_storage: Vec<u8>,
    status_storage: Vec<u8>,
}

impl VirtControlQueue {
    pub(super) fn new(queue_index: u16, queue_size: u16, hhdm: u64) -> Result<Self, &'static str> {
        if queue_size < 2 {
            return Err("virtio control queue requires at least 2 descriptors");
        }
        let queue_size = cmp::min(queue_size, VIRTIO_QUEUE_MAX_SIZE);
        let layout = VirtQueueLayout::new(queue_size as usize)?;

        Ok(Self {
            queue_index,
            queue_size,
            hhdm,
            memory: Box::new(AlignedVirtQueueMemory([0u8; VIRTIO_QUEUE_MEMORY_BYTES])),
            layout,
            next_avail_idx: 0,
            last_used_idx: 0,
            inflight: false,
            command_storage: vec![0u8; VIRTIO_CTRL_MAX_CMD_BYTES],
            status_storage: vec![0u8; 1],
        })
    }

    pub(super) fn queue_index(&self) -> u16 {
        self.queue_index
    }

    pub(super) fn queue_phys_addr(&self) -> Result<u64, &'static str> {
        let virt = self.memory.0.as_ptr() as u64;
        if virt < self.hhdm {
            return Err("virtio control queue address below hhdm");
        }
        Ok(virt - self.hhdm)
    }

    pub(super) fn prepare_command(&mut self, command: &[u8]) -> Result<(), &'static str> {
        if self.inflight {
            return Err("virtio control queue busy");
        }
        if command.is_empty() || command.len() > VIRTIO_CTRL_MAX_CMD_BYTES {
            return Err("virtio control command size invalid");
        }

        self.command_storage[..command.len()].copy_from_slice(command);
        self.status_storage[0] = VIRTIO_CTRL_ACK_PENDING;

        let cmd_addr = self.buffer_phys_addr(&self.command_storage)?;
        let status_addr = self.buffer_phys_addr(&self.status_storage)?;
        self.write_descriptor(0, cmd_addr, command.len() as u32, VIRTQ_DESC_F_NEXT, 1);
        self.write_descriptor(1, status_addr, 1, VIRTQ_DESC_F_WRITE, 0);
        self.publish_descriptor(0);
        self.inflight = true;
        Ok(())
    }

    pub(super) fn wait_completion(&mut self, timeout_spins: usize) -> Result<u8, &'static str> {
        if !self.inflight {
            return Err("virtio control queue has no inflight command");
        }

        let mut completed = false;
        for _ in 0..timeout_spins {
            if self.used_index() != self.last_used_idx {
                completed = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !completed {
            self.inflight = false;
            return Err("virtio control queue completion timeout");
        }

        let used_slot = (self.last_used_idx % self.queue_size) as usize;
        let used = self.used_elem(used_slot);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);
        self.inflight = false;
        if used.id != 0 {
            return Err("virtio control queue completion id mismatch");
        }
        Ok(self.status_storage[0])
    }

    fn buffer_phys_addr(&self, buf: &[u8]) -> Result<u64, &'static str> {
        let virt = buf.as_ptr() as u64;
        if virt < self.hhdm {
            return Err("virtio control buffer below hhdm");
        }
        Ok(virt - self.hhdm)
    }

    fn publish_descriptor(&mut self, desc_id: u16) {
        let slot = (self.next_avail_idx % self.queue_size) as usize;
        unsafe {
            let ring_ptr = self
                .memory
                .0
                .as_mut_ptr()
                .add(self.layout.avail_off + 4 + slot * size_of::<u16>())
                as *mut u16;
            write_volatile(ring_ptr, desc_id);
            compiler_fence(Ordering::Release);
            self.next_avail_idx = self.next_avail_idx.wrapping_add(1);
            let idx_ptr = self.memory.0.as_mut_ptr().add(self.layout.avail_off + 2) as *mut u16;
            write_volatile(idx_ptr, self.next_avail_idx);
        }
    }

    fn used_index(&self) -> u16 {
        unsafe {
            let ptr = self.memory.0.as_ptr().add(self.layout.used_off + 2) as *const u16;
            read_volatile(ptr)
        }
    }

    fn used_elem(&self, slot: usize) -> VirtqUsedElem {
        unsafe {
            let ptr = self
                .memory
                .0
                .as_ptr()
                .add(self.layout.used_off + 4 + slot * size_of::<VirtqUsedElem>())
                as *const VirtqUsedElem;
            read_volatile(ptr)
        }
    }

    fn write_descriptor(&mut self, desc_id: u16, addr: u64, len: u32, flags: u16, next: u16) {
        unsafe {
            let ptr = self
                .memory
                .0
                .as_mut_ptr()
                .add(self.layout.desc_off + desc_id as usize * size_of::<VirtqDesc>())
                as *mut VirtqDesc;
            write_volatile(
                ptr,
                VirtqDesc {
                    addr,
                    len,
                    flags,
                    next,
                },
            );
        }
    }
}

#[inline(always)]
const fn align_up(value: usize, align: usize) -> usize {
    (value + (align - 1)) & !(align - 1)
}
