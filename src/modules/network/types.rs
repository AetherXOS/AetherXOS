use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacAddress {
    Ethernet([u8; 6]),
}

pub struct Packet {
    pub data: Vec<u8>,
}

pub trait NetworkInterface {
    fn send(&mut self, packet: Packet) -> Result<(), &'static str>;
    fn receive(&mut self) -> Result<Option<Packet>, &'static str>;
    fn mac(&self) -> MacAddress;
}
