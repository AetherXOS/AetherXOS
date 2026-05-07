use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacAddress {
    Ethernet([u8; 6]),
}

pub enum PacketData {
    /// Heap-allocated buffer.
    Buffer(Vec<u8>),
    /// Reference to a physical frame (Zero-Copy).
    Physical {
        addr: u64,
        len: usize,
    },
}

impl PacketData {
    pub fn len(&self) -> usize {
        match self {
            PacketData::Buffer(b) => b.len(),
            PacketData::Physical { len, .. } => *len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct Packet {
    pub data: PacketData,
}

pub trait NetworkInterface {
    fn send(&mut self, packet: Packet) -> Result<(), &'static str>;
    fn receive(&mut self) -> Result<Option<Packet>, &'static str>;
    fn mac(&self) -> MacAddress;
}
