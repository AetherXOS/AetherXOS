#[cfg(feature = "network_wireguard")]
use super::*;
#[cfg(feature = "network_wireguard")]
use state::WG_PEERS;

#[cfg(feature = "network_wireguard")]
pub(super) fn update_wg_peer_high_water(depth: usize) {
    let depth = depth as u64;
    let mut current = WG_ACTIVE_PEERS_HIGH_WATER.load(Ordering::Relaxed);
    while depth > current {
        match WG_ACTIVE_PEERS_HIGH_WATER.compare_exchange_weak(
            current,
            depth,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

#[cfg(feature = "network_wireguard")]
pub fn wireguard_add_peer(
    public_key: [u8; 32],
    endpoint_ipv4: [u8; 4],
    endpoint_port: u16,
) -> Result<u64, &'static str> {
    if endpoint_port == 0 {
        return Err("invalid wireguard endpoint port");
    }
    WG_ADD_PEER_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut peers = WG_PEERS.lock();
    if peers.len() >= crate::config::KernelConfig::network_wireguard_max_peers() {
        WG_DROP_CALLS.fetch_add(1, Ordering::Relaxed);
        return Err("wireguard peer table full");
    }

    let peer_id = WG_NEXT_PEER_ID.fetch_add(1, Ordering::Relaxed);
    peers.insert(
        peer_id,
        state::WireGuardPeer {
            id: peer_id,
            public_key,
            endpoint_ipv4,
            endpoint_port,
        },
    );
    update_wg_peer_high_water(peers.len());
    Ok(peer_id)
}

#[cfg(feature = "network_wireguard")]
pub fn wireguard_remove_peer(peer_id: u64) -> bool {
    WG_REMOVE_PEER_CALLS.fetch_add(1, Ordering::Relaxed);
    WG_PEERS.lock().remove(&peer_id).is_some()
}

#[cfg(feature = "network_wireguard")]
pub fn wireguard_peer_count() -> usize {
    WG_PEERS.lock().len()
}

#[cfg(feature = "network_wireguard")]
pub fn wireguard_encapsulate(peer_id: u64, payload: &[u8]) -> Result<Vec<u8>, &'static str> {
    WG_ENCAP_CALLS.fetch_add(1, Ordering::Relaxed);
    if !WG_PEERS.lock().contains_key(&peer_id) {
        WG_DROP_CALLS.fetch_add(1, Ordering::Relaxed);
        return Err("wireguard peer not found");
    }

    const WG_TAG: [u8; 4] = *b"WGBL";
    let mut out = Vec::with_capacity(4 + 8 + payload.len());
    out.extend_from_slice(&WG_TAG);
    out.extend_from_slice(&peer_id.to_le_bytes());
    out.extend_from_slice(payload);
    WG_BYTES_ENCAP.fetch_add(payload.len() as u64, Ordering::Relaxed);
    Ok(out)
}

#[cfg(feature = "network_wireguard")]
pub fn wireguard_decapsulate(packet: &[u8]) -> Result<(u64, Vec<u8>), &'static str> {
    WG_DECAP_CALLS.fetch_add(1, Ordering::Relaxed);
    if packet.len() < 12 {
        WG_DROP_CALLS.fetch_add(1, Ordering::Relaxed);
        return Err("wireguard packet too short");
    }
    if &packet[0..4] != b"WGBL" {
        WG_DROP_CALLS.fetch_add(1, Ordering::Relaxed);
        return Err("wireguard packet tag mismatch");
    }

    let mut peer_bytes = [0u8; 8];
    peer_bytes.copy_from_slice(&packet[4..12]);
    let peer_id = u64::from_le_bytes(peer_bytes);
    if !WG_PEERS.lock().contains_key(&peer_id) {
        WG_DROP_CALLS.fetch_add(1, Ordering::Relaxed);
        return Err("wireguard peer unknown");
    }

    let payload = packet[12..].to_vec();
    WG_BYTES_DECAP.fetch_add(payload.len() as u64, Ordering::Relaxed);
    Ok((peer_id, payload))
}
