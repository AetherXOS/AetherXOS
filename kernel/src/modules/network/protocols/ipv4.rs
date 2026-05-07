use crate::aether_packet;

aether_packet! {
    pub struct Ipv4Packet<'a> {
        bitfield ver_ihl_block : u8 {
            version: u8 = 4..8;
            ihl: u8 = 0..4;
        }
        tos: u8;
        total_len: u16;
        id: u16;
        frag_off: u16;
        ttl: u8;
        protocol: u8;
        checksum: u16;
        src_ip: u32;
        dst_ip: u32;
    }
}

impl<'a> Ipv4Packet<'a> {
    pub fn payload(&self) -> &'a [u8] {
        let offset = self.ihl() as usize * 4;
        &self.as_bytes()[offset..]
    }
}

pub struct Ipv4Handler;

impl Ipv4Handler {
    /// Process an incoming IPv4 packet with full validation.
    pub fn handle_packet(data: &[u8]) -> Result<(), &'static str> {
        let packet = Ipv4Packet::new(data).ok_or("IP packet too short")?;

        // 1. Validation (Absolute Zero-Overhead via Fast-Path Getters)
        if packet.version() != 4 { return Err("not an IPv4 packet"); }
        let ihl = packet.ihl() as usize * 4;
        if ihl < 20 || data.len() < ihl { return Err("invalid IP header length"); }
        if packet.ttl() == 0 { return Err("IP packet expired (TTL=0)"); }

        // 2. Dispatch
        let protocol = packet.protocol();
        let payload = packet.payload();
        
        crate::klog_trace!("[IPv4] {:?}", packet);

        match protocol {
            6 => Self::dispatch_tcp(payload),
            17 => Self::dispatch_udp(payload),
            1 => Self::dispatch_icmp(payload),
            p => {
                crate::klog_info!("[NET] Unsupported IP protocol {}", p);
                Ok(())
            }
        }
    }

    fn dispatch_tcp(_data: &[u8]) -> Result<(), &'static str> {
        crate::klog_info!("[NET] TCP segment received");
        Ok(())
    }

    fn dispatch_udp(_data: &[u8]) -> Result<(), &'static str> {
        crate::klog_info!("[NET] UDP datagram received");
        Ok(())
    }

    fn dispatch_icmp(_data: &[u8]) -> Result<(), &'static str> {
        crate::klog_info!("[NET] ICMP packet received");
        Ok(())
    }
}
