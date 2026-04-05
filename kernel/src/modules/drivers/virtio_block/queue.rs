use alloc::vec::Vec;

/// Virtio block request types
pub(super) const VIRTIO_BLK_T_IN: u32 = 0;
pub(super) const VIRTIO_BLK_T_OUT: u32 = 1;

/// Queue depth: power of two, <= the device's reported QUEUE_SIZE.
const QUEUE_SIZE: usize = 64;
const QUEUE_ALIGN: usize = 4096;
pub(super) const VIRTQ_QUEUE_SIZE: usize = QUEUE_SIZE;

/// A single entry in the descriptor table (VirtIO 1.1 section 2.6).
#[repr(C)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// Available ring (driver -> device).
#[repr(C)]
#[allow(dead_code)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE],
}

/// Used ring element.
#[repr(C, packed)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

/// Used ring (device -> driver).
#[repr(C)]
#[allow(dead_code)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; QUEUE_SIZE],
}

/// Virtio block request header (16 bytes).
#[repr(C, packed)]
pub(super) struct VirtioBlkReq {
    pub type_: u32,
    pub reserved: u32,
    pub sector: u64,
}

/// In-memory virtqueue: descriptor table + avail + used rings.
pub(super) struct VirtQueue {
    pub desc: Vec<u8>,
    pub avail: Vec<u8>,
    pub used: Vec<u8>,
    pub last_used: u16,
    free_head: u16,
}

impl VirtQueue {
    pub(super) fn new() -> Self {
        let desc_size = QUEUE_SIZE * core::mem::size_of::<VirtqDesc>();
        let avail_size = 4 + QUEUE_SIZE * 2;
        let used_size = 4 + QUEUE_SIZE * core::mem::size_of::<VirtqUsedElem>() + 2;

        let mut desc = alloc::vec![0u8; desc_size];
        let avail = alloc::vec![0u8; avail_size];
        let used = alloc::vec![0u8; used_size.max(QUEUE_ALIGN)];

        // Build free-list chain: desc[i].next = i + 1.
        for i in 0..(QUEUE_SIZE - 1) {
            let off = i * 16;
            desc[off + 14..off + 16].copy_from_slice(&(i as u16 + 1).to_le_bytes());
        }

        Self {
            desc,
            avail,
            used,
            last_used: 0,
            free_head: 0,
        }
    }

    pub(super) fn phys(v: &[u8]) -> u64 {
        v.as_ptr() as u64
    }

    pub(super) fn desc_write(&mut self, idx: usize, addr: u64, len: u32, flags: u16, next: u16) {
        let off = idx * 16;
        self.desc[off..off + 8].copy_from_slice(&addr.to_le_bytes());
        self.desc[off + 8..off + 12].copy_from_slice(&len.to_le_bytes());
        self.desc[off + 12..off + 14].copy_from_slice(&flags.to_le_bytes());
        self.desc[off + 14..off + 16].copy_from_slice(&next.to_le_bytes());
    }

    fn avail_idx(&self) -> u16 {
        u16::from_le_bytes([self.avail[2], self.avail[3]])
    }

    pub(super) fn avail_push(&mut self, head: u16) {
        let idx = self.avail_idx();
        let slot = (idx as usize % QUEUE_SIZE) * 2 + 4;
        self.avail[slot..slot + 2].copy_from_slice(&head.to_le_bytes());
        self.avail[2..4].copy_from_slice(&idx.wrapping_add(1).to_le_bytes());
    }

    pub(super) fn used_idx(&self) -> u16 {
        u16::from_le_bytes([self.used[2], self.used[3]])
    }

    pub(super) fn alloc_desc(&mut self) -> Option<u16> {
        let head = self.free_head;
        if head as usize >= QUEUE_SIZE {
            return None;
        }
        let off = head as usize * 16 + 14;
        self.free_head = u16::from_le_bytes([self.desc[off], self.desc[off + 1]]);
        Some(head)
    }

    pub(super) fn free_desc(&mut self, idx: u16) {
        let off = idx as usize * 16 + 14;
        self.desc[off..off + 2].copy_from_slice(&self.free_head.to_le_bytes());
        self.free_head = idx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn alloc_desc_exhausts_and_returns_none() {
        let mut q = VirtQueue::new();
        for _ in 0..VIRTQ_QUEUE_SIZE {
            assert!(q.alloc_desc().is_some());
        }
        assert!(q.alloc_desc().is_none());
    }

    #[test_case]
    fn free_desc_returns_descriptor_to_pool() {
        let mut q = VirtQueue::new();
        let d0 = q.alloc_desc().expect("first descriptor");
        let _d1 = q.alloc_desc().expect("second descriptor");
        q.free_desc(d0);

        let recycled = q.alloc_desc().expect("recycled descriptor");
        assert_eq!(recycled, d0);
    }

    #[test_case]
    fn avail_push_advances_ring_index() {
        let mut q = VirtQueue::new();
        assert_eq!(q.avail_idx(), 0);
        q.avail_push(3);
        assert_eq!(q.avail_idx(), 1);
        q.avail_push(4);
        assert_eq!(q.avail_idx(), 2);
    }
}
