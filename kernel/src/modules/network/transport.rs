#[cfg(feature = "network_transport")]
use alloc::vec::Vec;

#[cfg(feature = "network_transport")]
pub use crate::modules::network::{
    FilterAction, FilterProtocol, PacketFilterRule, TcpListener, TcpStream, UdpDatagram, UdpSocket,
};

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct TransportSnapshot {
    pub udp_bind_calls: u64,
    pub udp_send_calls: u64,
    pub udp_send_drops: u64,
    pub udp_recv_calls: u64,
    pub udp_recv_hits: u64,
    pub udp_queue_high_water: u64,
    pub tcp_listen_calls: u64,
    pub tcp_connect_calls: u64,
    pub tcp_accept_calls: u64,
    pub tcp_accept_hits: u64,
    pub tcp_send_calls: u64,
    pub tcp_send_drops: u64,
    pub tcp_recv_calls: u64,
    pub tcp_recv_hits: u64,
    pub tcp_queue_high_water: u64,
    pub dns_register_calls: u64,
    pub dns_resolve_calls: u64,
    pub dns_resolve_hits: u64,
    pub filter_register_calls: u64,
    pub filter_remove_calls: u64,
    pub filter_clear_calls: u64,
    pub filter_eval_calls: u64,
    pub filter_eval_allow: u64,
    pub filter_eval_drop: u64,
    pub udp_send_latency_p50_ticks: u64,
    pub udp_send_latency_p95_ticks: u64,
    pub udp_send_latency_p99_ticks: u64,
    pub udp_recv_latency_p50_ticks: u64,
    pub udp_recv_latency_p95_ticks: u64,
    pub udp_recv_latency_p99_ticks: u64,
    pub tcp_send_latency_p50_ticks: u64,
    pub tcp_send_latency_p95_ticks: u64,
    pub tcp_send_latency_p99_ticks: u64,
    pub tcp_recv_latency_p50_ticks: u64,
    pub tcp_recv_latency_p95_ticks: u64,
    pub tcp_recv_latency_p99_ticks: u64,
    pub udp_queue_saturation_percent: u64,
    pub tcp_queue_saturation_percent: u64,
    pub queue_saturation_percent: u64,
    pub udp_saturation_class: QueueSaturationClass,
    pub tcp_saturation_class: QueueSaturationClass,
    pub saturation_class: QueueSaturationClass,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueSaturationClass {
    Nominal,
    Elevated,
    High,
    Critical,
}

#[cfg(feature = "network_transport")]
const QUEUE_DEPTH_LIMIT_DEFAULT: u64 = 256;
#[cfg(feature = "network_transport")]
const SATURATION_SCALE_PERCENT: u64 = 100;
#[cfg(feature = "network_transport")]
const SATURATION_ELEVATED_THRESHOLD: u64 = 50;
#[cfg(feature = "network_transport")]
const SATURATION_HIGH_THRESHOLD: u64 = 80;
#[cfg(feature = "network_transport")]
const SATURATION_CRITICAL_THRESHOLD: u64 = 95;

#[cfg(feature = "network_transport")]
fn saturation_percent(high_water: u64, limit: u64) -> u64 {
    let safe_limit = core::cmp::max(limit, 1);
    core::cmp::min(
        high_water.saturating_mul(SATURATION_SCALE_PERCENT) / safe_limit,
        SATURATION_SCALE_PERCENT,
    )
}

#[cfg(feature = "network_transport")]
fn classify_saturation(percent: u64) -> QueueSaturationClass {
    if percent >= SATURATION_CRITICAL_THRESHOLD {
        QueueSaturationClass::Critical
    } else if percent >= SATURATION_HIGH_THRESHOLD {
        QueueSaturationClass::High
    } else if percent >= SATURATION_ELEVATED_THRESHOLD {
        QueueSaturationClass::Elevated
    } else {
        QueueSaturationClass::Nominal
    }
}

#[cfg(feature = "network_transport")]
pub fn snapshot() -> TransportSnapshot {
    let net = crate::modules::network::bridge::stats();
    let udp_saturation_percent =
        saturation_percent(net.udp_queue_high_water, QUEUE_DEPTH_LIMIT_DEFAULT);
    let tcp_saturation_percent =
        saturation_percent(net.tcp_queue_high_water, QUEUE_DEPTH_LIMIT_DEFAULT);
    let saturation_percent = core::cmp::max(udp_saturation_percent, tcp_saturation_percent);

    TransportSnapshot {
        udp_bind_calls: net.udp_bind_calls,
        udp_send_calls: net.udp_send_calls,
        udp_send_drops: net.udp_send_drops,
        udp_recv_calls: net.udp_recv_calls,
        udp_recv_hits: net.udp_recv_hits,
        udp_queue_high_water: net.udp_queue_high_water,
        tcp_listen_calls: net.tcp_listen_calls,
        tcp_connect_calls: net.tcp_connect_calls,
        tcp_accept_calls: net.tcp_accept_calls,
        tcp_accept_hits: net.tcp_accept_hits,
        tcp_send_calls: net.tcp_send_calls,
        tcp_send_drops: net.tcp_send_drops,
        tcp_recv_calls: net.tcp_recv_calls,
        tcp_recv_hits: net.tcp_recv_hits,
        tcp_queue_high_water: net.tcp_queue_high_water,
        dns_register_calls: net.dns_register_calls,
        dns_resolve_calls: net.dns_resolve_calls,
        dns_resolve_hits: net.dns_resolve_hits,
        filter_register_calls: net.filter_register_calls,
        filter_remove_calls: net.filter_remove_calls,
        filter_clear_calls: net.filter_clear_calls,
        filter_eval_calls: net.filter_eval_calls,
        filter_eval_allow: net.filter_eval_allow,
        filter_eval_drop: net.filter_eval_drop,
        udp_send_latency_p50_ticks: net.udp_send_latency_p50_ticks,
        udp_send_latency_p95_ticks: net.udp_send_latency_p95_ticks,
        udp_send_latency_p99_ticks: net.udp_send_latency_p99_ticks,
        udp_recv_latency_p50_ticks: net.udp_recv_latency_p50_ticks,
        udp_recv_latency_p95_ticks: net.udp_recv_latency_p95_ticks,
        udp_recv_latency_p99_ticks: net.udp_recv_latency_p99_ticks,
        tcp_send_latency_p50_ticks: net.tcp_send_latency_p50_ticks,
        tcp_send_latency_p95_ticks: net.tcp_send_latency_p95_ticks,
        tcp_send_latency_p99_ticks: net.tcp_send_latency_p99_ticks,
        tcp_recv_latency_p50_ticks: net.tcp_recv_latency_p50_ticks,
        tcp_recv_latency_p95_ticks: net.tcp_recv_latency_p95_ticks,
        tcp_recv_latency_p99_ticks: net.tcp_recv_latency_p99_ticks,
        udp_queue_saturation_percent: udp_saturation_percent,
        tcp_queue_saturation_percent: tcp_saturation_percent,
        queue_saturation_percent: saturation_percent,
        udp_saturation_class: classify_saturation(udp_saturation_percent),
        tcp_saturation_class: classify_saturation(tcp_saturation_percent),
        saturation_class: classify_saturation(saturation_percent),
    }
}

#[cfg(feature = "network_transport")]
pub fn udp_bind(port: u16) -> Result<UdpSocket, &'static str> {
    crate::modules::network::udp_bind(port)
}

#[cfg(feature = "network_transport")]
pub fn tcp_listen(port: u16) -> Result<TcpListener, &'static str> {
    crate::modules::network::tcp_listen(port)
}

#[cfg(feature = "network_transport")]
pub fn tcp_connect(local_port: u16, remote_port: u16) -> Result<TcpStream, &'static str> {
    crate::modules::network::tcp_connect(local_port, remote_port)
}

#[cfg(feature = "network_transport")]
pub fn dns_register(name: &str, ipv4: [u8; 4]) -> Result<(), &'static str> {
    crate::modules::network::dns_register(name, ipv4)
}

#[cfg(feature = "network_transport")]
pub fn dns_resolve(name: &str) -> Option<[u8; 4]> {
    crate::modules::network::dns_resolve(name)
}

#[cfg(feature = "network_transport")]
pub fn register_packet_filter(
    protocol: FilterProtocol,
    src_port: Option<u16>,
    dst_port: Option<u16>,
    max_payload_len: Option<usize>,
    action: FilterAction,
) -> Result<u64, &'static str> {
    crate::modules::network::register_packet_filter(
        protocol,
        src_port,
        dst_port,
        max_payload_len,
        action,
    )
}

#[cfg(feature = "network_transport")]
pub fn remove_packet_filter(id: u64) -> bool {
    crate::modules::network::remove_packet_filter(id)
}

#[cfg(feature = "network_transport")]
pub fn clear_packet_filters() {
    crate::modules::network::clear_packet_filters();
}

#[cfg(feature = "network_transport")]
pub fn packet_filter_rules() -> alloc::vec::Vec<PacketFilterRule> {
    crate::modules::network::packet_filter_rules()
}

#[cfg(feature = "network_transport")]
pub fn udp_send_batch(socket: &UdpSocket, dst_port: u16, payloads: &[&[u8]]) -> (usize, usize) {
    let mut sent_packets = 0usize;
    let mut sent_bytes = 0usize;
    for payload in payloads {
        match socket.send_to(dst_port, payload) {
            Ok(bytes) => {
                sent_packets += 1;
                sent_bytes += bytes;
            }
            Err(_) => break,
        }
    }
    (sent_packets, sent_bytes)
}

#[cfg(feature = "network_transport")]
pub fn tcp_send_batch(stream: &TcpStream, payloads: &[&[u8]]) -> (usize, usize) {
    let mut sent_chunks = 0usize;
    let mut sent_bytes = 0usize;
    for payload in payloads {
        match stream.send(payload) {
            Ok(bytes) => {
                sent_chunks += 1;
                sent_bytes += bytes;
            }
            Err(_) => break,
        }
    }
    (sent_chunks, sent_bytes)
}

#[cfg(feature = "network_transport")]
pub fn udp_recv_batch(socket: &UdpSocket, max_packets: usize) -> Vec<UdpDatagram> {
    let mut out = Vec::new();
    for _ in 0..max_packets {
        let Some(datagram) = socket.recv() else {
            break;
        };
        out.push(datagram);
    }
    out
}

#[cfg(feature = "network_transport")]
/// Poll all active transport-layer state machines (UDP/TCP receive queues,
/// keepalive timers).  Called by the network-poll kernel service daemon.
pub fn poll_all() {
    // Advance smoltcp-style timers if the network bridge is active.
    #[cfg(feature = "networking")]
    {
        let _ = crate::modules::network::bridge::poll_smoltcp_runtime();
    }
    // Non-smoltcp path: no-op for now (raw packet ring is drained at IRQ time).
}

pub fn tcp_recv_batch(stream: &TcpStream, max_chunks: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for _ in 0..max_chunks {
        let Some(chunk) = stream.recv() else {
            break;
        };
        out.push(chunk);
    }
    out
}

#[cfg(all(test, feature = "network_transport"))]
mod tests {
    use super::*;

    #[test_case]
    fn saturation_classification_thresholds() {
        assert_eq!(classify_saturation(0), QueueSaturationClass::Nominal);
        assert_eq!(
            classify_saturation(SATURATION_ELEVATED_THRESHOLD),
            QueueSaturationClass::Elevated
        );
        assert_eq!(
            classify_saturation(SATURATION_HIGH_THRESHOLD),
            QueueSaturationClass::High
        );
        assert_eq!(
            classify_saturation(SATURATION_CRITICAL_THRESHOLD),
            QueueSaturationClass::Critical
        );
    }

    #[test_case]
    fn snapshot_saturation_metrics_are_bounded() {
        let s = snapshot();
        assert!(s.udp_queue_saturation_percent <= SATURATION_SCALE_PERCENT);
        assert!(s.tcp_queue_saturation_percent <= SATURATION_SCALE_PERCENT);
        assert!(s.queue_saturation_percent <= SATURATION_SCALE_PERCENT);
    }
}
