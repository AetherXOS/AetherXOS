use super::*;
use state::PACKET_FILTERS;

#[cfg(feature = "network_transport")]
pub(super) fn apply_filters(
    protocol: FilterProtocol,
    src_port: Option<u16>,
    dst_port: Option<u16>,
    payload_len: usize,
) -> Result<(), &'static str> {
    FILTER_EVAL_CALLS.fetch_add(1, Ordering::Relaxed);
    let filters = PACKET_FILTERS.lock();
    for rule in filters.iter() {
        if rule.protocol != FilterProtocol::Any && rule.protocol != protocol {
            continue;
        }
        if let Some(expected) = rule.src_port {
            if src_port != Some(expected) {
                continue;
            }
        }
        if let Some(expected) = rule.dst_port {
            if dst_port != Some(expected) {
                continue;
            }
        }
        if let Some(limit) = rule.max_payload_len {
            if payload_len > limit {
                continue;
            }
        }

        match rule.action {
            FilterAction::Allow => {
                FILTER_EVAL_ALLOW.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            FilterAction::Drop => {
                FILTER_EVAL_DROP.fetch_add(1, Ordering::Relaxed);
                return Err("packet dropped by filter");
            }
        }
    }

    FILTER_EVAL_ALLOW.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[cfg(feature = "network_transport")]
pub fn register_packet_filter(
    protocol: FilterProtocol,
    src_port: Option<u16>,
    dst_port: Option<u16>,
    max_payload_len: Option<usize>,
    action: FilterAction,
) -> Result<u64, &'static str> {
    FILTER_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut filters = PACKET_FILTERS.lock();
    if filters.len() >= crate::config::KernelConfig::network_filter_rule_limit() {
        return Err("filter rule table full");
    }
    let id = FILTER_NEXT_ID.fetch_add(1, Ordering::Relaxed);
    filters.push(PacketFilterRule {
        id,
        protocol,
        src_port,
        dst_port,
        max_payload_len,
        action,
    });
    Ok(id)
}

#[cfg(feature = "network_transport")]
pub fn remove_packet_filter(id: u64) -> bool {
    FILTER_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut filters = PACKET_FILTERS.lock();
    let before = filters.len();
    filters.retain(|rule| rule.id != id);
    filters.len() != before
}

#[cfg(feature = "network_transport")]
pub fn clear_packet_filters() {
    FILTER_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
    PACKET_FILTERS.lock().clear();
}

#[cfg(feature = "network_transport")]
pub fn packet_filter_rules() -> Vec<PacketFilterRule> {
    PACKET_FILTERS.lock().clone()
}
