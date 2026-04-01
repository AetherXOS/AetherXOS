use super::*;

pub fn smoltcp_ethernet_address(mac: MacAddress) -> smoltcp::wire::EthernetAddress {
    runtime::smoltcp_ethernet_address(mac)
}

pub fn init_smoltcp_bridge(nic: &dyn NetworkInterface) -> smoltcp::wire::EthernetAddress {
    runtime::init_smoltcp_bridge(nic)
}

pub fn init_smoltcp_runtime(nic: &dyn NetworkInterface) -> Result<(), &'static str> {
    runtime::init_smoltcp_runtime(nic)
}

pub fn reinitialize_smoltcp_runtime(nic: &dyn NetworkInterface) -> Result<(), &'static str> {
    runtime::reinitialize_smoltcp_runtime(nic)
}

pub fn poll_smoltcp_runtime() -> bool {
    runtime::poll_smoltcp_runtime()
}

pub fn force_poll_once() -> bool {
    runtime::force_poll_once()
}

pub fn ingest_raw_ethernet_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    runtime::ingest_raw_ethernet_frame(frame)
}

pub fn ingest_raw_ethernet_frames(frames: Vec<Vec<u8>>) -> usize {
    runtime::ingest_raw_ethernet_frames(frames)
}