use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressurePolicy {
    Drop,
    Defer,
    ForcePoll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackpressurePolicyTable {
    pub loopback: BackpressurePolicy,
    #[cfg(feature = "network_transport")]
    pub udp: BackpressurePolicy,
    #[cfg(feature = "network_transport")]
    pub tcp: BackpressurePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkAlertThresholds {
    pub min_health_score: u64,
    pub max_drops: u64,
    pub max_queue_high_water: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkAlertReport {
    pub health_breach: bool,
    pub drops_breach: bool,
    pub queue_breach: bool,
    pub breach_count: u8,
}

pub fn backpressure_policy_table() -> BackpressurePolicyTable {
    BackpressurePolicyTable {
        loopback: policy_from_u64(LOOPBACK_BACKPRESSURE_POLICY.load(Ordering::Relaxed)),
        #[cfg(feature = "network_transport")]
        udp: policy_from_u64(UDP_BACKPRESSURE_POLICY.load(Ordering::Relaxed)),
        #[cfg(feature = "network_transport")]
        tcp: policy_from_u64(TCP_BACKPRESSURE_POLICY.load(Ordering::Relaxed)),
    }
}

pub fn set_backpressure_policy_table(table: BackpressurePolicyTable) {
    LOOPBACK_BACKPRESSURE_POLICY.store(policy_to_u64(table.loopback), Ordering::Relaxed);
    #[cfg(feature = "network_transport")]
    UDP_BACKPRESSURE_POLICY.store(policy_to_u64(table.udp), Ordering::Relaxed);
    #[cfg(feature = "network_transport")]
    TCP_BACKPRESSURE_POLICY.store(policy_to_u64(table.tcp), Ordering::Relaxed);
}

pub fn network_alert_thresholds() -> NetworkAlertThresholds {
    NetworkAlertThresholds {
        min_health_score: ALERT_MIN_HEALTH_SCORE.load(Ordering::Relaxed),
        max_drops: ALERT_MAX_DROPS.load(Ordering::Relaxed),
        max_queue_high_water: ALERT_MAX_QUEUE_HIGH_WATER.load(Ordering::Relaxed),
    }
}

pub fn set_network_alert_thresholds(thresholds: NetworkAlertThresholds) {
    ALERT_MIN_HEALTH_SCORE.store(thresholds.min_health_score, Ordering::Relaxed);
    ALERT_MAX_DROPS.store(thresholds.max_drops, Ordering::Relaxed);
    ALERT_MAX_QUEUE_HIGH_WATER.store(thresholds.max_queue_high_water, Ordering::Relaxed);
}

pub fn evaluate_network_alerts() -> NetworkAlertReport {
    let stats = bridge_stats();
    let thresholds = network_alert_thresholds();

    let total_drops = stats
        .loopback_send_drops
        .saturating_add(stats.udp_send_drops)
        .saturating_add(stats.tcp_send_drops);
    let peak_queue = core::cmp::max(
        stats.loopback_queue_high_water,
        core::cmp::max(stats.udp_queue_high_water, stats.tcp_queue_high_water),
    );

    compute_network_alert_report(
        stats.smoltcp_health_score,
        total_drops,
        peak_queue,
        thresholds,
    )
}