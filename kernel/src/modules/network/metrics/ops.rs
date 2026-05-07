use super::super::*;

pub use super::snapshot::{
    bridge_stats, NetworkBridgeStats, recommended_runtime_health_action,
    runtime_health_report, NetworkRuntimeHealthAction, NetworkRuntimeHealthReport,
};

pub fn current_latency_tick() -> u64 {
    crate::kernel::watchdog::global_tick()
}

pub fn update_tcp_high_water(depth: usize) {
    support::update_high_water(&state_counters::TCP_QUEUE_HIGH_WATER, depth);
}

pub fn update_udp_high_water(depth: usize) {
    support::update_high_water(&state_counters::UDP_QUEUE_HIGH_WATER, depth);
}

pub fn update_loopback_high_water(depth: usize) {
    support::update_high_water(&state_counters::LOOPBACK_QUEUE_HIGH_WATER, depth);
}

pub fn record_udp_send_latency(delta: u64) {
    support::record_latency_bucket(
        delta,
        &state_counters::UDP_SEND_LAT_BUCKET_0,
        &state_counters::UDP_SEND_LAT_BUCKET_1,
        &state_counters::UDP_SEND_LAT_BUCKET_2_3,
        &state_counters::UDP_SEND_LAT_BUCKET_4_7,
        &state_counters::UDP_SEND_LAT_BUCKET_GE8,
    );
}

pub fn record_udp_recv_latency(delta: u64) {
    support::record_latency_bucket(
        delta,
        &state_counters::UDP_RECV_LAT_BUCKET_0,
        &state_counters::UDP_RECV_LAT_BUCKET_1,
        &state_counters::UDP_RECV_LAT_BUCKET_2_3,
        &state_counters::UDP_RECV_LAT_BUCKET_4_7,
        &state_counters::UDP_RECV_LAT_BUCKET_GE8,
    );
}

pub fn record_tcp_send_latency(delta: u64) {
    support::record_latency_bucket(
        delta,
        &state_counters::TCP_SEND_LAT_BUCKET_0,
        &state_counters::TCP_SEND_LAT_BUCKET_1,
        &state_counters::TCP_SEND_LAT_BUCKET_2_3,
        &state_counters::TCP_SEND_LAT_BUCKET_4_7,
        &state_counters::TCP_SEND_LAT_BUCKET_GE8,
    );
}

pub fn record_tcp_recv_latency(delta: u64) {
    support::record_latency_bucket(
        delta,
        &state_counters::TCP_RECV_LAT_BUCKET_0,
        &state_counters::TCP_RECV_LAT_BUCKET_1,
        &state_counters::TCP_RECV_LAT_BUCKET_2_3,
        &state_counters::TCP_RECV_LAT_BUCKET_4_7,
        &state_counters::TCP_RECV_LAT_BUCKET_GE8,
    );
}
