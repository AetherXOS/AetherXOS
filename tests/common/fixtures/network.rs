pub const TEST_MAC_ADDRESS: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
pub const TEST_IP_ADDRESS_V4: [u8; 4] = [192, 168, 1, 100];
pub const TEST_SUBNET_MASK: [u8; 4] = [255, 255, 255, 0];
pub const TEST_GATEWAY: [u8; 4] = [192, 168, 1, 1];
pub const TEST_DNS: [u8; 4] = [8, 8, 8, 8];

pub struct NetworkFixture {
    pub mac: [u8; 6],
    pub ip_v4: [u8; 4],
    pub subnet_mask: [u8; 4],
    pub gateway: [u8; 4],
    pub mtu: usize,
}

impl NetworkFixture {
    pub fn new() -> Self {
        Self {
            mac: TEST_MAC_ADDRESS,
            ip_v4: TEST_IP_ADDRESS_V4,
            subnet_mask: TEST_SUBNET_MASK,
            gateway: TEST_GATEWAY,
            mtu: 1500,
        }
    }

    pub fn with_mtu(mut self, mtu: usize) -> Self {
        self.mtu = mtu;
        self
    }

    pub fn with_ip(mut self, ip: [u8; 4]) -> Self {
        self.ip_v4 = ip;
        self
    }
}

impl Default for NetworkFixture {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_ethernet_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len() + 4);
    frame.extend_from_slice(&[0xFF; 6]);
    frame.extend_from_slice(&TEST_MAC_ADDRESS);
    frame.extend_from_slice(&0x0800u16.to_be_bytes());
    frame.extend_from_slice(payload);
    frame.extend_from_slice(&[0u8; 4]);
    frame
}

pub fn create_ip_packet(protocol: u8, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(20 + payload.len());
    packet.push(0x45);
    packet.push(0x00);
    let total_len = (20 + payload.len()) as u16;
    packet.extend_from_slice(&total_len.to_be_bytes());
    packet.extend_from_slice(&0x0001u16.to_be_bytes());
    packet.extend_from_slice(&0x4000u16.to_be_bytes());
    packet.push(64);
    packet.push(protocol);
    packet.extend_from_slice(&0x0000u16.to_be_bytes());
    packet.extend_from_slice(&TEST_IP_ADDRESS_V4);
    packet.extend_from_slice(&TEST_GATEWAY);
    packet.extend_from_slice(payload);
    packet
}

pub fn create_tcp_segment(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
    let mut segment = Vec::with_capacity(20 + payload.len());
    segment.extend_from_slice(&src_port.to_be_bytes());
    segment.extend_from_slice(&dst_port.to_be_bytes());
    segment.extend_from_slice(&0x00000001u32.to_be_bytes());
    segment.extend_from_slice(&0x00000001u32.to_be_bytes());
    segment.push(0x50);
    segment.push(0x02);
    segment.extend_from_slice(&0xFFFFu16.to_be_bytes());
    segment.extend_from_slice(&0x0000u16.to_be_bytes());
    segment.extend_from_slice(payload);
    segment
}

pub fn create_udp_datagram(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
    let mut datagram = Vec::with_capacity(8 + payload.len());
    datagram.extend_from_slice(&src_port.to_be_bytes());
    datagram.extend_from_slice(&dst_port.to_be_bytes());
    let len = (8 + payload.len()) as u16;
    datagram.extend_from_slice(&len.to_be_bytes());
    datagram.extend_from_slice(&0x0000u16.to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

pub fn create_dns_query(domain: &str) -> Vec<u8> {
    let mut query = Vec::new();
    query.extend_from_slice(&0x0001u16.to_be_bytes());
    query.extend_from_slice(&0x0100u16.to_be_bytes());
    query.extend_from_slice(&0x0001u16.to_be_bytes());
    query.extend_from_slice(&0x0000u16.to_be_bytes());
    query.extend_from_slice(&0x0000u16.to_be_bytes());
    query.extend_from_slice(&0x0000u16.to_be_bytes());
    
    for part in domain.split('.') {
        query.push(part.len() as u8);
        query.extend_from_slice(part.as_bytes());
    }
    query.push(0x00);
    query.extend_from_slice(&0x0001u16.to_be_bytes());
    query.extend_from_slice(&0x0001u16.to_be_bytes());
    
    query
}
