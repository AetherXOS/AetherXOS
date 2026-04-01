pub(super) const DEFAULT_DOORBELL_STRIDE: u64 = 4;

#[inline]
pub(super) fn sq_doorbell_offset(queue_id: u16, dstrd: u64) -> u64 {
    0x1000 + (2 * queue_id as u64 * dstrd)
}

#[inline]
pub(super) fn cq_doorbell_offset(queue_id: u16, dstrd: u64) -> u64 {
    0x1000 + ((2 * queue_id as u64 + 1) * dstrd)
}

pub(super) const NVME_CMD_READ: u8 = 0x02;
pub(super) const NVME_CMD_WRITE: u8 = 0x01;

pub(super) fn build_io_sqe(
    opcode: u8,
    cid: u16,
    nsid: u32,
    prp1: u64,
    slba: u64,
    nlb: u16,
) -> [u32; 16] {
    let cdw0: u32 = (opcode as u32) | ((cid as u32) << 16);

    let mut sqe = [0u32; 16];
    sqe[0] = cdw0;
    sqe[1] = nsid;
    sqe[6] = (prp1 & 0xFFFF_FFFF) as u32;
    sqe[7] = (prp1 >> 32) as u32;
    sqe[10] = (slba & 0xFFFF_FFFF) as u32;
    sqe[11] = (slba >> 32) as u32;
    sqe[12] = nlb as u32;
    sqe
}

pub(super) fn build_create_io_cq_sqe(cid: u16, qid: u16, phys: u64, size: u16) -> [u32; 16] {
    let mut sqe = [0u32; 16];
    sqe[0] = 0x05 | ((cid as u32) << 16);
    sqe[6] = (phys & 0xFFFF_FFFF) as u32;
    sqe[7] = (phys >> 32) as u32;
    sqe[10] = ((size - 1) as u32) | ((qid as u32) << 16);
    sqe[11] = 0x01;
    sqe
}

pub(super) fn build_create_io_sq_sqe(
    cid: u16,
    qid: u16,
    cqid: u16,
    phys: u64,
    size: u16,
) -> [u32; 16] {
    let mut sqe = [0u32; 16];
    sqe[0] = 0x01 | ((cid as u32) << 16);
    sqe[6] = (phys & 0xFFFF_FFFF) as u32;
    sqe[7] = (phys >> 32) as u32;
    sqe[10] = ((size - 1) as u32) | ((qid as u32) << 16);
    sqe[11] = ((cqid as u32) << 16) | 0x01;
    sqe
}

pub(super) const CQE_DW3_PHASE_BIT: u32 = 1 << 16;
pub(super) const CQE_DW3_CID_MASK: u32 = 0x0000_FFFF;
pub(super) const CQE_DW3_SF_SHIFT: u32 = 17;
pub(super) const CQE_DW3_SF_MASK: u32 = 0x7FFF;
