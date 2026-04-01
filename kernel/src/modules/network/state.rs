#[cfg(any(
    feature = "network_transport",
    feature = "network_http",
    feature = "network_wireguard"
))]
use alloc::collections::BTreeMap;
#[cfg(feature = "network_transport")]
use alloc::collections::VecDeque;
#[cfg(feature = "network_transport")]
use alloc::string::String;
#[cfg(feature = "network_http")]
use alloc::sync::Arc;
use alloc::vec::Vec;
#[cfg(any(
    feature = "network_transport",
    feature = "network_http",
    feature = "network_wireguard"
))]
use lazy_static::lazy_static;
#[cfg(any(
    feature = "network_transport",
    feature = "network_http",
    feature = "network_wireguard"
))]
use spin::Mutex;

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct UdpSocket {
    pub(super) local_port: u16,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterProtocol {
    Any,
    Raw,
    Udp,
    Tcp,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    Allow,
    Drop,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct PacketFilterRule {
    pub id: u64,
    pub protocol: FilterProtocol,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub max_payload_len: Option<usize>,
    pub action: FilterAction,
}

#[cfg(feature = "network_transport")]
lazy_static! {
    pub(super) static ref UDP_ENDPOINTS: Mutex<BTreeMap<u16, VecDeque<UdpDatagram>>> =
        Mutex::new(BTreeMap::new());
    pub(super) static ref TCP_LISTENERS: Mutex<BTreeMap<u16, ()>> = Mutex::new(BTreeMap::new());
    pub(super) static ref TCP_PENDING_ACCEPT: Mutex<BTreeMap<u16, VecDeque<u16>>> =
        Mutex::new(BTreeMap::new());
    pub(super) static ref TCP_STREAM_QUEUES: Mutex<BTreeMap<u16, VecDeque<Vec<u8>>>> =
        Mutex::new(BTreeMap::new());
    pub(super) static ref DNS_TABLE: Mutex<BTreeMap<String, [u8; 4]>> = Mutex::new(BTreeMap::new());
    pub(super) static ref PACKET_FILTERS: Mutex<Vec<PacketFilterRule>> = Mutex::new(Vec::new());
}

#[cfg(feature = "network_wireguard")]
#[derive(Debug, Clone)]
pub struct WireGuardPeer {
    pub id: u64,
    pub public_key: [u8; 32],
    pub endpoint_ipv4: [u8; 4],
    pub endpoint_port: u16,
}

#[cfg(feature = "network_wireguard")]
lazy_static! {
    pub(super) static ref WG_PEERS: Mutex<BTreeMap<u64, WireGuardPeer>> =
        Mutex::new(BTreeMap::new());
}

#[cfg(feature = "network_http")]
lazy_static! {
    pub(super) static ref HTTP_STATIC_ASSETS: Mutex<BTreeMap<String, HttpStaticAsset>> =
        Mutex::new(BTreeMap::new());
}

#[cfg(feature = "network_http")]
#[derive(Debug, Clone)]
pub struct HttpStaticAsset {
    pub path: String,
    pub content_type: String,
    pub body: Arc<Vec<u8>>,
    pub etag: u64,
}

#[cfg(feature = "network_http")]
#[derive(Debug, Clone)]
pub struct HttpSendfileView {
    pub body: Arc<Vec<u8>>,
    pub offset: usize,
    pub len: usize,
}

#[cfg(feature = "network_http")]
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<HttpSendfileView>,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct TcpListener {
    pub(super) local_port: u16,
}

#[cfg(feature = "network_transport")]
#[derive(Debug, Clone, Copy)]
pub struct TcpStream {
    pub(super) local_port: u16,
    pub(super) peer_port: u16,
}

pub(super) fn clear_runtime_state_tables() {
    #[cfg(feature = "network_transport")]
    {
        UDP_ENDPOINTS.lock().clear();
        TCP_LISTENERS.lock().clear();
        TCP_PENDING_ACCEPT.lock().clear();
        TCP_STREAM_QUEUES.lock().clear();
        DNS_TABLE.lock().clear();
        PACKET_FILTERS.lock().clear();
    }
    #[cfg(feature = "network_wireguard")]
    WG_PEERS.lock().clear();
    #[cfg(feature = "network_http")]
    HTTP_STATIC_ASSETS.lock().clear();
}
