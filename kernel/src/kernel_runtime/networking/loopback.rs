#[cfg(feature = "networking")]
pub(crate) struct KernelLoopbackNic;

#[cfg(feature = "networking")]
impl KernelLoopbackNic {
    pub(crate) const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "networking")]
impl aethercore::modules::network::NetworkInterface for KernelLoopbackNic {
    fn send(&mut self, packet: aethercore::modules::network::Packet) -> Result<(), &'static str> {
        aethercore::modules::network::ingest_raw_ethernet_frame(packet.data)
    }

    fn receive(&mut self) -> Result<Option<aethercore::modules::network::Packet>, &'static str> {
        Ok(None)
    }

    fn mac(&self) -> aethercore::modules::network::MacAddress {
        aethercore::modules::network::MacAddress::Ethernet([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
    }
}
