//! ARP (Address Resolution Protocol) Handler for AetherXOS.
//! RFC 826 compliant.

use alloc::collections::BTreeMap;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

use crate::aether_packet;

aether_packet! {
    pub struct ArpPacket<'a> {
        hw_type: u16;
        proto_type: u16;
        hw_addr_len: u8;
        proto_addr_len: u8;
        op: u16;
        sender_mac: [u8; 6];
        sender_ip: u32;
        target_mac: [u8; 6];
        target_ip: u32;
    }
}

pub struct ArpCache {
    entries: BTreeMap<u32, MacAddress>,
}

impl ArpCache {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new() }
    }

    pub fn update(&mut self, ip: u32, mac: MacAddress) {
        crate::klog_info!("[ARP] Updating entry: {}.{}.{}.{} -> {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            (ip >> 24) & 0xFF, (ip >> 16) & 0xFF, (ip >> 8) & 0xFF, ip & 0xFF,
            mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]);
        self.entries.insert(ip, mac);
    }

    pub fn lookup(&self, ip: u32) -> Option<MacAddress> {
        self.entries.get(&ip).copied()
    }
}

pub static ARP_CACHE: Mutex<ArpCache> = Mutex::new(ArpCache {
    entries: BTreeMap::new(),
});

pub fn handle_arp_packet(packet_raw: &[u8]) {
    let packet = match ArpPacket::new(packet_raw) {
        Some(p) => p,
        None => return,
    };
    
    crate::klog_trace!("[ARP RX] {:?}", packet);

    let op = packet.op();
    let sender_ip = packet.sender_ip();
    let sender_mac = MacAddress(packet.sender_mac());

    match op {
        1 => { // Request
            crate::klog_info!("[ARP] Received request from {}.{}.{}.{}", 
                (sender_ip >> 24) & 0xFF, (sender_ip >> 16) & 0xFF, (sender_ip >> 8) & 0xFF, sender_ip & 0xFF);
            ARP_CACHE.lock().update(sender_ip, sender_mac);
        }
        2 => { // Reply
            crate::klog_info!("[ARP] Received reply from {}.{}.{}.{}", 
                (sender_ip >> 24) & 0xFF, (sender_ip >> 16) & 0xFF, (sender_ip >> 8) & 0xFF, sender_ip & 0xFF);
            ARP_CACHE.lock().update(sender_ip, sender_mac);
        }
        _ => {}
    }
}
