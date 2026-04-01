use super::*;
use alloc::collections::VecDeque;

#[cfg(feature = "network_transport")]
pub fn udp_bind(port: u16) -> Result<UdpSocket, &'static str> {
    if port == 0 {
        return Err("invalid udp port");
    }
    UDP_BIND_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut endpoints = UDP_ENDPOINTS.lock();
    endpoints.entry(port).or_insert_with(VecDeque::new);
    Ok(UdpSocket { local_port: port })
}