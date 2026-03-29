pub fn init_with_nic(
    nic: &dyn crate::modules::network::NetworkInterface,
) -> Result<(), &'static str> {
    crate::modules::libnet::policy::ensure_l34_enabled()?;
    crate::modules::network::bridge::init_smoltcp_runtime(nic)
}

pub fn poll_transport_once() -> bool {
    if crate::modules::libnet::policy::ensure_l34_enabled().is_err() {
        return false;
    }
    crate::modules::network::bridge::poll_smoltcp_runtime()
}

pub fn set_polling_enabled(enabled: bool) {
    if crate::modules::libnet::policy::ensure_l34_enabled().is_err() {
        return;
    }
    crate::modules::network::bridge::set_runtime_polling_enabled(enabled);
}

pub fn set_poll_interval_ticks(interval: u64) {
    if crate::modules::libnet::policy::ensure_l34_enabled().is_err() {
        return;
    }
    crate::modules::network::bridge::set_runtime_poll_interval_ticks(interval);
}

#[cfg(feature = "network_transport")]
pub use crate::modules::network::transport::TransportSnapshot;
#[cfg(feature = "network_transport")]
pub use crate::modules::network::transport::{
    FilterAction, FilterProtocol, PacketFilterRule, UdpDatagram,
};

#[cfg(feature = "network_transport")]
pub trait DatagramSocket {
    fn local_port(&self) -> u16;
    fn send_to(&self, dst_port: u16, payload: &[u8]) -> Result<usize, &'static str>;
    fn recv(&self) -> Option<UdpDatagram>;
}

#[cfg(feature = "network_transport")]
pub trait StreamSocket {
    fn local_port(&self) -> u16;
    fn peer_port(&self) -> u16;
    fn send(&self, payload: &[u8]) -> Result<usize, &'static str>;
    fn recv(&self) -> Option<alloc::vec::Vec<u8>>;
}

#[cfg(feature = "network_transport")]
pub trait CustomSocketFactory {
    type Datagram: DatagramSocket;
    type Stream: StreamSocket;

    fn udp_bind(&self, local_port: u16) -> Result<Self::Datagram, &'static str>;
    fn tcp_connect(&self, local_port: u16, remote_port: u16) -> Result<Self::Stream, &'static str>;
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSocketFactory;

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct LibUdpSocket {
    inner: crate::modules::network::transport::UdpSocket,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct LibTcpListener {
    inner: crate::modules::network::transport::TcpListener,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct LibTcpStream {
    inner: crate::modules::network::transport::TcpStream,
}

#[cfg(feature = "network_transport")]
fn ensure_transport_policy() -> Result<(), &'static str> {
    crate::modules::libnet::policy::ensure_l34_enabled()
}

#[cfg(feature = "network_transport")]
pub fn udp_bind(local_port: u16) -> Result<LibUdpSocket, &'static str> {
    ensure_transport_policy()?;
    Ok(LibUdpSocket {
        inner: crate::modules::network::transport::udp_bind(local_port)?,
    })
}

#[cfg(feature = "network_transport")]
pub fn tcp_listen(local_port: u16) -> Result<LibTcpListener, &'static str> {
    ensure_transport_policy()?;
    Ok(LibTcpListener {
        inner: crate::modules::network::transport::tcp_listen(local_port)?,
    })
}

#[cfg(feature = "network_transport")]
pub fn tcp_connect(local_port: u16, remote_port: u16) -> Result<LibTcpStream, &'static str> {
    ensure_transport_policy()?;
    Ok(LibTcpStream {
        inner: crate::modules::network::transport::tcp_connect(local_port, remote_port)?,
    })
}

#[cfg(feature = "network_transport")]
impl CustomSocketFactory for DefaultSocketFactory {
    type Datagram = LibUdpSocket;
    type Stream = LibTcpStream;

    fn udp_bind(&self, local_port: u16) -> Result<Self::Datagram, &'static str> {
        udp_bind(local_port)
    }

    fn tcp_connect(&self, local_port: u16, remote_port: u16) -> Result<Self::Stream, &'static str> {
        tcp_connect(local_port, remote_port)
    }
}

#[cfg(feature = "network_transport")]
impl LibTcpListener {
    pub fn local_port(&self) -> u16 {
        self.inner.local_port()
    }

    pub fn accept(&self) -> Option<LibTcpStream> {
        self.inner.accept().map(|inner| LibTcpStream { inner })
    }
}

#[cfg(feature = "network_transport")]
impl LibUdpSocket {
    pub fn send_batch(&self, dst_port: u16, payloads: &[&[u8]]) -> (usize, usize) {
        crate::modules::network::transport::udp_send_batch(&self.inner, dst_port, payloads)
    }

    pub fn recv_batch(&self, max_packets: usize) -> alloc::vec::Vec<UdpDatagram> {
        crate::modules::network::transport::udp_recv_batch(&self.inner, max_packets)
    }
}

#[cfg(feature = "network_transport")]
impl DatagramSocket for LibUdpSocket {
    fn local_port(&self) -> u16 {
        self.inner.local_port()
    }

    fn send_to(&self, dst_port: u16, payload: &[u8]) -> Result<usize, &'static str> {
        self.inner.send_to(dst_port, payload)
    }

    fn recv(&self) -> Option<UdpDatagram> {
        self.inner.recv()
    }
}

#[cfg(feature = "network_transport")]
impl StreamSocket for LibTcpStream {
    fn local_port(&self) -> u16 {
        self.inner.local_port()
    }

    fn peer_port(&self) -> u16 {
        self.inner.peer_port()
    }

    fn send(&self, payload: &[u8]) -> Result<usize, &'static str> {
        self.inner.send(payload)
    }

    fn recv(&self) -> Option<alloc::vec::Vec<u8>> {
        self.inner.recv()
    }
}

#[cfg(feature = "network_transport")]
impl LibTcpStream {
    pub fn send_batch(&self, payloads: &[&[u8]]) -> (usize, usize) {
        crate::modules::network::transport::tcp_send_batch(&self.inner, payloads)
    }

    pub fn recv_batch(&self, max_chunks: usize) -> alloc::vec::Vec<alloc::vec::Vec<u8>> {
        crate::modules::network::transport::tcp_recv_batch(&self.inner, max_chunks)
    }
}

#[cfg(feature = "network_transport")]
pub fn transport_snapshot() -> TransportSnapshot {
    crate::modules::network::transport::snapshot()
}

#[cfg(feature = "network_transport")]
pub fn register_packet_filter(
    protocol: FilterProtocol,
    src_port: Option<u16>,
    dst_port: Option<u16>,
    max_payload_len: Option<usize>,
    action: FilterAction,
) -> Result<u64, &'static str> {
    ensure_transport_policy()?;
    crate::modules::network::transport::register_packet_filter(
        protocol,
        src_port,
        dst_port,
        max_payload_len,
        action,
    )
}

#[cfg(feature = "network_transport")]
pub fn remove_packet_filter(id: u64) -> bool {
    if ensure_transport_policy().is_err() {
        return false;
    }
    crate::modules::network::transport::remove_packet_filter(id)
}

#[cfg(feature = "network_transport")]
pub fn clear_packet_filters() {
    if ensure_transport_policy().is_err() {
        return;
    }
    crate::modules::network::transport::clear_packet_filters();
}

#[cfg(feature = "network_transport")]
pub fn packet_filter_rules() -> alloc::vec::Vec<PacketFilterRule> {
    if ensure_transport_policy().is_err() {
        return alloc::vec::Vec::new();
    }
    crate::modules::network::transport::packet_filter_rules()
}

#[cfg(feature = "network_transport")]
pub fn dns_register(name: &str, ipv4: [u8; 4]) -> Result<(), &'static str> {
    ensure_transport_policy()?;
    crate::modules::network::transport::dns_register(name, ipv4)
}

#[cfg(feature = "network_transport")]
pub fn dns_resolve(name: &str) -> Option<[u8; 4]> {
    if ensure_transport_policy().is_err() {
        return None;
    }
    crate::modules::network::transport::dns_resolve(name)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "network_transport")]
    use super::*;

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn transport_snapshot_counters_are_monotonic() {
        let before = transport_snapshot();

        let sender = udp_bind(31000).expect("bind sender");
        let receiver = udp_bind(31001).expect("bind receiver");
        let _ = sender.send_to(receiver.local_port(), b"mono");
        let _ = receiver.recv();

        let listener = tcp_listen(32001).expect("listen");
        let client = tcp_connect(32000, listener.local_port()).expect("connect");
        let server = listener.accept().expect("accept");
        let _ = client.send(b"contract");
        let _ = server.recv();

        let after = transport_snapshot();

        assert!(after.udp_bind_calls >= before.udp_bind_calls);
        assert!(after.udp_send_calls >= before.udp_send_calls);
        assert!(after.udp_recv_calls >= before.udp_recv_calls);
        assert!(after.tcp_listen_calls >= before.tcp_listen_calls);
        assert!(after.tcp_connect_calls >= before.tcp_connect_calls);
        assert!(after.tcp_send_calls >= before.tcp_send_calls);
        assert!(after.tcp_recv_calls >= before.tcp_recv_calls);
    }
}
