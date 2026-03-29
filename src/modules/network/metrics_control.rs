use super::*;

pub fn set_runtime_polling_enabled(enabled: bool) {
    SMOLTCP_RUNTIME_CONTROL_UPDATES.fetch_add(1, Ordering::Relaxed);
    SMOLTCP_RUNTIME_POLL_ENABLED.store(if enabled { 1 } else { 0 }, Ordering::Relaxed);
}

pub fn runtime_polling_enabled() -> bool {
    SMOLTCP_RUNTIME_POLL_ENABLED.load(Ordering::Relaxed) != 0
}

pub fn set_runtime_poll_interval_ticks(interval: u64) {
    let clamped = core::cmp::max(
        interval,
        crate::config::KernelConfig::network_runtime_poll_interval_min(),
    );
    SMOLTCP_RUNTIME_CONTROL_UPDATES.fetch_add(1, Ordering::Relaxed);
    SMOLTCP_POLL_INTERVAL_TICKS.store(clamped, Ordering::Relaxed);
}

pub fn runtime_poll_interval_ticks() -> u64 {
    SMOLTCP_POLL_INTERVAL_TICKS.load(Ordering::Relaxed)
}

pub fn reset_runtime_stats() {
    reset_counter(&SMOLTCP_BRIDGE_INITS, 0);
    reset_counter(&SMOLTCP_POLLS, 0);
    reset_counter(&SMOLTCP_RX_FRAMES, 0);
    reset_counter(&SMOLTCP_TX_FRAMES, 0);
    reset_counter(&SMOLTCP_TICKS, 0);
    reset_counter(&SMOLTCP_INIT_ERRORS, 0);
    reset_counter(&SMOLTCP_POLL_ERRORS, 0);
    reset_counter(&SMOLTCP_POLL_SKIPS, 0);
    reset_counter(&SMOLTCP_FORCE_POLLS, 0);
    reset_counter(&SMOLTCP_REINITIALIZE_CALLS, 0);
    reset_counter(
        &SMOLTCP_POLL_INTERVAL_TICKS,
        crate::config::KernelConfig::network_runtime_poll_interval_min(),
    );
    reset_counter(&SMOLTCP_RUNTIME_CONTROL_UPDATES, 0);
    reset_counter(&BACKPRESSURE_DROP_ACTIONS, 0);
    reset_counter(&BACKPRESSURE_DEFER_ACTIONS, 0);
    reset_counter(&BACKPRESSURE_FORCE_POLL_ACTIONS, 0);
    reset_counter(&LOOPBACK_BACKPRESSURE_POLICY, BACKPRESSURE_POLICY_DROP_RAW);
    #[cfg(feature = "network_transport")]
    reset_counter(&UDP_BACKPRESSURE_POLICY, BACKPRESSURE_POLICY_DROP_RAW);
    #[cfg(feature = "network_transport")]
    reset_counter(&TCP_BACKPRESSURE_POLICY, BACKPRESSURE_POLICY_DROP_RAW);
    reset_counter(&ALERT_MIN_HEALTH_SCORE, ALERT_MIN_HEALTH_SCORE_DEFAULT);
    reset_counter(&ALERT_MAX_DROPS, ALERT_LIMIT_OPEN_MAX);
    reset_counter(&ALERT_MAX_QUEUE_HIGH_WATER, ALERT_LIMIT_OPEN_MAX);
    reset_counters(
        &[
            &LOOPBACK_SEND_CALLS,
            &LOOPBACK_SEND_DROPS,
            &LOOPBACK_RECEIVE_CALLS,
            &LOOPBACK_RECEIVE_HITS,
            &LOOPBACK_QUEUE_HIGH_WATER,
            &UDP_BIND_CALLS,
            &UDP_SEND_CALLS,
            &UDP_SEND_DROPS,
            &UDP_RECV_CALLS,
            &UDP_RECV_HITS,
            &UDP_QUEUE_HIGH_WATER,
            &TCP_LISTEN_CALLS,
            &TCP_CONNECT_CALLS,
            &TCP_ACCEPT_CALLS,
            &TCP_ACCEPT_HITS,
            &TCP_SEND_CALLS,
            &TCP_SEND_DROPS,
            &TCP_RECV_CALLS,
            &TCP_RECV_HITS,
            &TCP_QUEUE_HIGH_WATER,
        ],
        0,
    );
    reset_latency_buckets(
        &UDP_SEND_LAT_BUCKET_0,
        &UDP_SEND_LAT_BUCKET_1,
        &UDP_SEND_LAT_BUCKET_2_3,
        &UDP_SEND_LAT_BUCKET_4_7,
        &UDP_SEND_LAT_BUCKET_GE8,
    );
    reset_latency_buckets(
        &UDP_RECV_LAT_BUCKET_0,
        &UDP_RECV_LAT_BUCKET_1,
        &UDP_RECV_LAT_BUCKET_2_3,
        &UDP_RECV_LAT_BUCKET_4_7,
        &UDP_RECV_LAT_BUCKET_GE8,
    );
    reset_latency_buckets(
        &TCP_SEND_LAT_BUCKET_0,
        &TCP_SEND_LAT_BUCKET_1,
        &TCP_SEND_LAT_BUCKET_2_3,
        &TCP_SEND_LAT_BUCKET_4_7,
        &TCP_SEND_LAT_BUCKET_GE8,
    );
    reset_latency_buckets(
        &TCP_RECV_LAT_BUCKET_0,
        &TCP_RECV_LAT_BUCKET_1,
        &TCP_RECV_LAT_BUCKET_2_3,
        &TCP_RECV_LAT_BUCKET_4_7,
        &TCP_RECV_LAT_BUCKET_GE8,
    );
    reset_counters(
        &[
            &DNS_REGISTER_CALLS,
            &DNS_RESOLVE_CALLS,
            &DNS_RESOLVE_HITS,
            &FILTER_REGISTER_CALLS,
            &FILTER_REMOVE_CALLS,
            &FILTER_CLEAR_CALLS,
            &FILTER_EVAL_CALLS,
            &FILTER_EVAL_ALLOW,
            &FILTER_EVAL_DROP,
        ],
        0,
    );
    FILTER_NEXT_ID.store(1, Ordering::Relaxed);
    #[cfg(feature = "network_wireguard")]
    reset_counters(
        &[
            &WG_ADD_PEER_CALLS,
            &WG_REMOVE_PEER_CALLS,
            &WG_ENCAP_CALLS,
            &WG_DECAP_CALLS,
            &WG_DROP_CALLS,
            &WG_BYTES_ENCAP,
            &WG_BYTES_DECAP,
            &WG_ACTIVE_PEERS_HIGH_WATER,
        ],
        0,
    );
    #[cfg(feature = "network_wireguard")]
    WG_NEXT_PEER_ID.store(1, Ordering::Relaxed);
    #[cfg(feature = "network_http")]
    reset_counters(
        &[
            &HTTP_REGISTER_CALLS,
            &HTTP_REMOVE_CALLS,
            &HTTP_REQUEST_CALLS,
            &HTTP_RESP_200,
            &HTTP_RESP_304,
            &HTTP_RESP_404,
            &HTTP_SENDFILE_CALLS,
            &HTTP_BYTES_SERVED,
        ],
        0,
    );
    clear_runtime_state_tables();
}