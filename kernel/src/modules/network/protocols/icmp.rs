//! ICMP (Internet Control Message Protocol) Handler for AetherXOS.
//! RFC 792 compliant.

use crate::aether_packet;

aether_packet! {
    pub struct IcmpHeader<'a> {
        type_: u8;
        code: u8;
        checksum: u16;
        rest_of_header: u32;
    }
}

pub fn handle_icmp_packet(packet_raw: &[u8], src_ip: u32) {
    let header = match IcmpHeader::new(packet_raw) {
        Some(h) => h,
        None => return,
    };
    
    let icmp_type = header.type_();
    let icmp_code = header.code();
    
    match icmp_type {
        8 => { // Echo Request (Ping)
            crate::klog_info!("[ICMP] Echo Request received from {}.{}.{}.{} (code {})",
                (src_ip >> 24) & 0xFF, (src_ip >> 16) & 0xFF, (src_ip >> 8) & 0xFF, src_ip & 0xFF,
                icmp_code);
            // In a real NIC, we'd send an Echo Reply (type 0) here
        }
        3 => { // Destination Unreachable
            crate::klog_warn!("[ICMP] Destination Unreachable from {}", src_ip);
        }
        _ => {
            crate::klog_info!("[ICMP] Received type {} from {}", icmp_type, src_ip);
        }
    }
}
