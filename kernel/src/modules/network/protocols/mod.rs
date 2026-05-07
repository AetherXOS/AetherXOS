//! Network Protocols Entry Point for AetherXOS.

pub mod ipv4;
pub mod arp;
pub mod icmp;
pub mod tcp;

/// Route an incoming packet to the appropriate protocol handler.
pub fn route_packet(data: &[u8]) -> Result<(), &'static str> {
    if data.is_empty() { return Err("empty packet"); }
    
    // Simplified EtherType detection
    // In a real NIC driver, this would be determined by the Ethernet header
    ipv4::Ipv4Handler::handle_packet(data)
}
