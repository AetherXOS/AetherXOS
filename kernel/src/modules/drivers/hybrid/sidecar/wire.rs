use alloc::vec::Vec;

const SIDECAR_WIRE_VERSION: u16 = 1;
const SIDECAR_WIRE_HEADER_BYTES: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioQueueSelector {
    Control,
    Completion,
    Tx,
    Rx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideCarOpcode {
    Probe,
    Attach,
    Detach,
    MapDma,
    UnmapDma,
    NotifyQueue,
    InjectIrq,
    AckIrq,
    Reset,
    QueryStatus,
}

impl SideCarOpcode {
    pub const fn to_u16(self) -> u16 {
        match self {
            SideCarOpcode::Probe => 0,
            SideCarOpcode::Attach => 1,
            SideCarOpcode::Detach => 2,
            SideCarOpcode::MapDma => 3,
            SideCarOpcode::UnmapDma => 4,
            SideCarOpcode::NotifyQueue => 5,
            SideCarOpcode::InjectIrq => 6,
            SideCarOpcode::AckIrq => 7,
            SideCarOpcode::Reset => 8,
            SideCarOpcode::QueryStatus => 9,
        }
    }

    pub const fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(SideCarOpcode::Probe),
            1 => Some(SideCarOpcode::Attach),
            2 => Some(SideCarOpcode::Detach),
            3 => Some(SideCarOpcode::MapDma),
            4 => Some(SideCarOpcode::UnmapDma),
            5 => Some(SideCarOpcode::NotifyQueue),
            6 => Some(SideCarOpcode::InjectIrq),
            7 => Some(SideCarOpcode::AckIrq),
            8 => Some(SideCarOpcode::Reset),
            9 => Some(SideCarOpcode::QueryStatus),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarWireHeader {
    pub version: u16,
    pub opcode: SideCarOpcode,
    pub queue: VirtioQueueSelector,
    pub flags: u16,
    pub request_id: u64,
    pub vm_id: u16,
    pub device_id: u16,
    pub payload_len: u32,
    pub reserved: u32,
}

impl SideCarWireHeader {
    pub const fn new(
        opcode: SideCarOpcode,
        queue: VirtioQueueSelector,
        request_id: u64,
        vm_id: u16,
        payload_len: u32,
    ) -> Self {
        Self {
            version: SIDECAR_WIRE_VERSION,
            opcode,
            queue,
            flags: 0,
            request_id,
            vm_id,
            device_id: 0,
            payload_len,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideCarWireError {
    Truncated,
    UnsupportedVersion,
    InvalidOpcode,
    InvalidQueue,
    InvalidPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarInterruptRoute {
    pub host_vector: u32,
    pub guest_vector: u32,
    pub posted_interrupt: bool,
    pub coalescing_budget: usize,
}

impl SideCarInterruptRoute {
    pub const fn new(host_vector: u32, guest_vector: u32) -> Self {
        Self {
            host_vector,
            guest_vector,
            posted_interrupt: false,
            coalescing_budget: 1,
        }
    }

    pub fn with_posted_interrupt(mut self, enabled: bool) -> Self {
        self.posted_interrupt = enabled;
        self
    }

    pub fn with_coalescing_budget(mut self, budget: usize) -> Self {
        self.coalescing_budget = if budget == 0 { 1 } else { budget };
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideCarPayloadTag {
    Empty,
    DmaMap,
    QueueNotify,
    IrqRoute,
}

impl SideCarPayloadTag {
    const fn to_u8(self) -> u8 {
        match self {
            SideCarPayloadTag::Empty => 0,
            SideCarPayloadTag::DmaMap => 1,
            SideCarPayloadTag::QueueNotify => 2,
            SideCarPayloadTag::IrqRoute => 3,
        }
    }

    const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(SideCarPayloadTag::Empty),
            1 => Some(SideCarPayloadTag::DmaMap),
            2 => Some(SideCarPayloadTag::QueueNotify),
            3 => Some(SideCarPayloadTag::IrqRoute),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideCarPayload {
    Empty,
    DmaMap {
        iova_base: u64,
        length: u32,
        writable: bool,
    },
    QueueNotify {
        queue: VirtioQueueSelector,
        desc_count: u16,
        bytes: u32,
    },
    IrqRoute(SideCarInterruptRoute),
}

pub fn encode_payload(payload: &SideCarPayload) -> Vec<u8> {
    match payload {
        SideCarPayload::Empty => vec![SideCarPayloadTag::Empty.to_u8()],
        SideCarPayload::DmaMap {
            iova_base,
            length,
            writable,
        } => {
            let mut out = Vec::with_capacity(1 + 8 + 4 + 1);
            out.push(SideCarPayloadTag::DmaMap.to_u8());
            out.extend_from_slice(&iova_base.to_le_bytes());
            out.extend_from_slice(&length.to_le_bytes());
            out.push(u8::from(*writable));
            out
        }
        SideCarPayload::QueueNotify {
            queue,
            desc_count,
            bytes,
        } => {
            let mut out = Vec::with_capacity(1 + 2 + 2 + 4);
            out.push(SideCarPayloadTag::QueueNotify.to_u8());
            out.extend_from_slice(&queue_to_u16(*queue).to_le_bytes());
            out.extend_from_slice(&desc_count.to_le_bytes());
            out.extend_from_slice(&bytes.to_le_bytes());
            out
        }
        SideCarPayload::IrqRoute(route) => {
            let mut out = Vec::with_capacity(1 + 4 + 4 + 1 + 4);
            out.push(SideCarPayloadTag::IrqRoute.to_u8());
            out.extend_from_slice(&route.host_vector.to_le_bytes());
            out.extend_from_slice(&route.guest_vector.to_le_bytes());
            out.push(u8::from(route.posted_interrupt));
            out.extend_from_slice(&(route.coalescing_budget as u32).to_le_bytes());
            out
        }
    }
}

pub fn decode_payload(bytes: &[u8]) -> Result<SideCarPayload, SideCarWireError> {
    let Some(tag) = bytes.first().copied().and_then(SideCarPayloadTag::from_u8) else {
        return Err(SideCarWireError::InvalidPayload);
    };

    match tag {
        SideCarPayloadTag::Empty => Ok(SideCarPayload::Empty),
        SideCarPayloadTag::DmaMap => {
            if bytes.len() < 14 {
                return Err(SideCarWireError::Truncated);
            }
            let iova_base = u64::from_le_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
            ]);
            let length = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);
            let writable = bytes[13] != 0;
            Ok(SideCarPayload::DmaMap {
                iova_base,
                length,
                writable,
            })
        }
        SideCarPayloadTag::QueueNotify => {
            if bytes.len() < 9 {
                return Err(SideCarWireError::Truncated);
            }
            let queue_raw = u16::from_le_bytes([bytes[1], bytes[2]]);
            let Some(queue) = queue_from_u16(queue_raw) else {
                return Err(SideCarWireError::InvalidQueue);
            };
            let desc_count = u16::from_le_bytes([bytes[3], bytes[4]]);
            let msg_bytes = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
            Ok(SideCarPayload::QueueNotify {
                queue,
                desc_count,
                bytes: msg_bytes,
            })
        }
        SideCarPayloadTag::IrqRoute => {
            if bytes.len() < 14 {
                return Err(SideCarWireError::Truncated);
            }
            let host_vector = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
            let guest_vector = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
            let posted_interrupt = bytes[9] != 0;
            let coalescing_budget =
                u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
            Ok(SideCarPayload::IrqRoute(
                SideCarInterruptRoute::new(host_vector, guest_vector)
                    .with_posted_interrupt(posted_interrupt)
                    .with_coalescing_budget(coalescing_budget),
            ))
        }
    }
}

pub fn encode_wire_header(header: SideCarWireHeader) -> [u8; SIDECAR_WIRE_HEADER_BYTES] {
    let mut out = [0u8; SIDECAR_WIRE_HEADER_BYTES];
    out[0..2].copy_from_slice(&header.version.to_le_bytes());
    out[2..4].copy_from_slice(&header.opcode.to_u16().to_le_bytes());
    out[4..6].copy_from_slice(&queue_to_u16(header.queue).to_le_bytes());
    out[6..8].copy_from_slice(&header.flags.to_le_bytes());
    out[8..16].copy_from_slice(&header.request_id.to_le_bytes());
    out[16..18].copy_from_slice(&header.vm_id.to_le_bytes());
    out[18..20].copy_from_slice(&header.device_id.to_le_bytes());
    out[20..24].copy_from_slice(&header.payload_len.to_le_bytes());
    out[24..28].copy_from_slice(&header.reserved.to_le_bytes());
    out
}

pub fn decode_wire_header(bytes: &[u8]) -> Result<SideCarWireHeader, SideCarWireError> {
    if bytes.len() < SIDECAR_WIRE_HEADER_BYTES {
        return Err(SideCarWireError::Truncated);
    }

    let version = u16::from_le_bytes([bytes[0], bytes[1]]);
    if version != SIDECAR_WIRE_VERSION {
        return Err(SideCarWireError::UnsupportedVersion);
    }

    let opcode_raw = u16::from_le_bytes([bytes[2], bytes[3]]);
    let opcode = SideCarOpcode::from_u16(opcode_raw).ok_or(SideCarWireError::InvalidOpcode)?;

    let queue_raw = u16::from_le_bytes([bytes[4], bytes[5]]);
    let queue = queue_from_u16(queue_raw).ok_or(SideCarWireError::InvalidQueue)?;

    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    let request_id = u64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    let vm_id = u16::from_le_bytes([bytes[16], bytes[17]]);
    let device_id = u16::from_le_bytes([bytes[18], bytes[19]]);
    let payload_len = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let reserved = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

    Ok(SideCarWireHeader {
        version,
        opcode,
        queue,
        flags,
        request_id,
        vm_id,
        device_id,
        payload_len,
        reserved,
    })
}

pub fn build_wire_probe(vm_id: u16, request_id: u64) -> SideCarWireHeader {
    SideCarWireHeader::new(
        SideCarOpcode::Probe,
        VirtioQueueSelector::Control,
        request_id,
        vm_id,
        0,
    )
}

pub fn build_wire_notify(
    vm_id: u16,
    request_id: u64,
    queue: VirtioQueueSelector,
    payload_len: u32,
) -> SideCarWireHeader {
    SideCarWireHeader::new(SideCarOpcode::NotifyQueue, queue, request_id, vm_id, payload_len)
}

const fn queue_to_u16(queue: VirtioQueueSelector) -> u16 {
    match queue {
        VirtioQueueSelector::Control => 0,
        VirtioQueueSelector::Completion => 1,
        VirtioQueueSelector::Tx => 2,
        VirtioQueueSelector::Rx => 3,
    }
}

const fn queue_from_u16(raw: u16) -> Option<VirtioQueueSelector> {
    match raw {
        0 => Some(VirtioQueueSelector::Control),
        1 => Some(VirtioQueueSelector::Completion),
        2 => Some(VirtioQueueSelector::Tx),
        3 => Some(VirtioQueueSelector::Rx),
        _ => None,
    }
}
