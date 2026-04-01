#[cfg(feature = "networking")]
pub(crate) struct KernelLoopbackNic;

#[cfg(feature = "networking")]
impl KernelLoopbackNic {
    pub(crate) const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "networking")]
impl hypercore::modules::network::NetworkInterface for KernelLoopbackNic {
    fn send(&mut self, packet: hypercore::modules::network::Packet) -> Result<(), &'static str> {
        hypercore::modules::network::ingest_raw_ethernet_frame(packet.data)
    }

    fn receive(&mut self) -> Result<Option<hypercore::modules::network::Packet>, &'static str> {
        Ok(None)
    }

    fn mac(&self) -> hypercore::modules::network::MacAddress {
        hypercore::modules::network::MacAddress::Ethernet([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
    }
}
