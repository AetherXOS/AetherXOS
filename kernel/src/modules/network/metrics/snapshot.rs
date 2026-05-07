
pub use super::bridge_stats_type::NetworkBridgeStats;
pub use super::health_snapshot::{self, NetworkRuntimeHealthAction, NetworkRuntimeHealthReport};
use super::latency_snapshot;
use super::bridge_stats_builder;

pub fn bridge_stats() -> NetworkBridgeStats {
    let health = health_snapshot::collect_runtime_health_snapshot();
    let latency = latency_snapshot::collect_latency_snapshot();

    bridge_stats_builder::build_bridge_stats(&health, &latency)
}

pub fn runtime_health_report() -> NetworkRuntimeHealthReport {
    let health = health_snapshot::collect_runtime_health_snapshot();
    health_snapshot::evaluate_runtime_health(health)
}

pub fn recommended_runtime_health_action() -> NetworkRuntimeHealthAction {
    health_snapshot::recommended_runtime_action(runtime_health_report())
}