use super::*;
use alloc::string::ToString;

#[cfg(feature = "network_transport")]
pub fn dns_register(name: &str, ipv4: [u8; 4]) -> Result<(), &'static str> {
    if name.is_empty() || name.len() > 255 {
        return Err("invalid dns name");
    }
    DNS_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    DNS_TABLE.lock().insert(name.to_string(), ipv4);
    Ok(())
}

#[cfg(feature = "network_transport")]
pub fn dns_resolve(name: &str) -> Option<[u8; 4]> {
    DNS_RESOLVE_CALLS.fetch_add(1, Ordering::Relaxed);
    let result = DNS_TABLE.lock().get(name).copied();
    if result.is_some() {
        DNS_RESOLVE_HITS.fetch_add(1, Ordering::Relaxed);
    }
    result
}