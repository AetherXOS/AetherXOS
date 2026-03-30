use super::*;
#[path = "metrics_snapshot.rs"]
mod metrics_snapshot;

pub use metrics_snapshot::{bridge_stats, NetworkBridgeStats};
pub use metrics_snapshot::{
    recommended_runtime_health_action, runtime_health_report, NetworkRuntimeHealthAction,
    NetworkRuntimeHealthReport,
};

pub(super) fn update_loopback_high_water(depth: usize) {
    update_high_water(&LOOPBACK_QUEUE_HIGH_WATER, depth);
}

#[cfg(feature = "network_transport")]
pub(super) fn update_udp_high_water(depth: usize) {
    update_high_water(&UDP_QUEUE_HIGH_WATER, depth);
}

#[cfg(feature = "network_transport")]
pub(super) fn update_tcp_high_water(depth: usize) {
    update_high_water(&TCP_QUEUE_HIGH_WATER, depth);
}

#[cfg(feature = "network_transport")]
#[inline(always)]
pub(super) fn current_latency_tick() -> u64 {
    crate::kernel::watchdog::global_tick()
}


