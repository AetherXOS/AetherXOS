//! Network config validation — queue limits, SLO, TLS, FD ranges.

use crate::build_cfg::config_types::NetworkConfig;

const VALID_TLS_PROFILES: &[&str] = &["Minimal", "Balanced", "Strict"];

pub fn validate(c: &NetworkConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_TLS_PROFILES.contains(&c.tls_policy_profile.as_str()) {
        e.push(format!(
            "network.tls_policy_profile '{}' invalid, expected one of {:?}",
            c.tls_policy_profile, VALID_TLS_PROFILES
        ));
    }
    if c.loopback_queue_limit == 0 || c.loopback_queue_limit > 65536 {
        e.push(format!(
            "network.loopback_queue_limit {} out of range [1, 65536]",
            c.loopback_queue_limit
        ));
    }
    if c.udp_queue_limit == 0 || c.udp_queue_limit > 65536 {
        e.push(format!(
            "network.udp_queue_limit {} out of range [1, 65536]",
            c.udp_queue_limit
        ));
    }
    if c.tcp_queue_limit == 0 || c.tcp_queue_limit > 65536 {
        e.push(format!(
            "network.tcp_queue_limit {} out of range [1, 65536]",
            c.tcp_queue_limit
        ));
    }
    if c.filter_rule_limit > 16384 {
        e.push(format!(
            "network.filter_rule_limit {} exceeds max 16384",
            c.filter_rule_limit
        ));
    }
    if c.posix_ephemeral_start < 1024 {
        e.push(format!(
            "network.posix_ephemeral_start {} must be >= 1024",
            c.posix_ephemeral_start
        ));
    }
    if c.blocking_recv_retries == 0 || c.blocking_recv_retries > 1_000_000 {
        e.push(format!(
            "network.blocking_recv_retries {} out of range [1, 1000000]",
            c.blocking_recv_retries
        ));
    }
    if c.slo_sample_interval == 0 {
        e.push("network.slo_sample_interval must be > 0".to_string());
    }
    if c.slo_log_interval_multiplier == 0 {
        e.push("network.slo_log_interval_multiplier must be > 0".to_string());
    }

    e
}
