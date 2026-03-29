use super::*;

#[path = "metrics_bridge_stats_type.rs"]
mod metrics_bridge_stats_type;
#[path = "metrics_bridge_stats_builder.rs"]
mod metrics_bridge_stats_builder;
#[path = "metrics_health_snapshot.rs"]
mod metrics_health_snapshot;
#[path = "metrics_latency_snapshot.rs"]
mod metrics_latency_snapshot;

pub use metrics_bridge_stats_type::NetworkBridgeStats;

pub fn bridge_stats() -> NetworkBridgeStats {
    let health = metrics_health_snapshot::collect_runtime_health_snapshot();
    let latency = metrics_latency_snapshot::collect_latency_snapshot();

    metrics_bridge_stats_builder::build_bridge_stats(&health, &latency)
}