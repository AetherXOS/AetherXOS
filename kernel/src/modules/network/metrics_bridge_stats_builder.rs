use super::*;

pub(super) fn build_bridge_stats(
    health: &metrics_health_snapshot::RuntimeHealthSnapshot,
    latency: &metrics_latency_snapshot::LatencySnapshot,
) -> NetworkBridgeStats {
    NetworkBridgeStats {
        smoltcp_bridge_inits: SMOLTCP_BRIDGE_INITS.load(Ordering::Relaxed),
        smoltcp_polls: health.polls,
        smoltcp_rx_frames: SMOLTCP_RX_FRAMES.load(Ordering::Relaxed),
        smoltcp_tx_frames: SMOLTCP_TX_FRAMES.load(Ordering::Relaxed),
        smoltcp_runtime_ready: runtime::runtime_ready(),
        smoltcp_runtime_poll_enabled: SMOLTCP_RUNTIME_POLL_ENABLED.load(Ordering::Relaxed) != 0,
        smoltcp_init_errors: health.init_errors,
        smoltcp_poll_errors: health.poll_errors,
        smoltcp_poll_skips: SMOLTCP_POLL_SKIPS.load(Ordering::Relaxed),
        smoltcp_runtime_control_updates: SMOLTCP_RUNTIME_CONTROL_UPDATES.load(Ordering::Relaxed),
        smoltcp_poll_interval_ticks: SMOLTCP_POLL_INTERVAL_TICKS.load(Ordering::Relaxed),
        smoltcp_force_polls: SMOLTCP_FORCE_POLLS.load(Ordering::Relaxed),
        smoltcp_reinitialize_calls: SMOLTCP_REINITIALIZE_CALLS.load(Ordering::Relaxed),
        smoltcp_health_score: health.score,
        backpressure_drop_actions: BACKPRESSURE_DROP_ACTIONS.load(Ordering::Relaxed),
        backpressure_defer_actions: BACKPRESSURE_DEFER_ACTIONS.load(Ordering::Relaxed),
        backpressure_force_poll_actions: BACKPRESSURE_FORCE_POLL_ACTIONS.load(Ordering::Relaxed),
        loopback_backpressure_policy: LOOPBACK_BACKPRESSURE_POLICY.load(Ordering::Relaxed),
        #[cfg(feature = "network_transport")]
        udp_backpressure_policy: UDP_BACKPRESSURE_POLICY.load(Ordering::Relaxed),
        #[cfg(feature = "network_transport")]
        tcp_backpressure_policy: TCP_BACKPRESSURE_POLICY.load(Ordering::Relaxed),
        loopback_send_calls: LOOPBACK_SEND_CALLS.load(Ordering::Relaxed),
        loopback_send_drops: LOOPBACK_SEND_DROPS.load(Ordering::Relaxed),
        loopback_receive_calls: LOOPBACK_RECEIVE_CALLS.load(Ordering::Relaxed),
        loopback_receive_hits: LOOPBACK_RECEIVE_HITS.load(Ordering::Relaxed),
        loopback_queue_high_water: LOOPBACK_QUEUE_HIGH_WATER.load(Ordering::Relaxed),
        udp_bind_calls: UDP_BIND_CALLS.load(Ordering::Relaxed),
        udp_send_calls: latency.udp_send_calls,
        udp_send_drops: UDP_SEND_DROPS.load(Ordering::Relaxed),
        udp_recv_calls: latency.udp_recv_calls,
        udp_recv_hits: UDP_RECV_HITS.load(Ordering::Relaxed),
        udp_queue_high_water: UDP_QUEUE_HIGH_WATER.load(Ordering::Relaxed),
        tcp_listen_calls: TCP_LISTEN_CALLS.load(Ordering::Relaxed),
        tcp_connect_calls: TCP_CONNECT_CALLS.load(Ordering::Relaxed),
        tcp_accept_calls: TCP_ACCEPT_CALLS.load(Ordering::Relaxed),
        tcp_accept_hits: TCP_ACCEPT_HITS.load(Ordering::Relaxed),
        tcp_send_calls: latency.tcp_send_calls,
        tcp_send_drops: TCP_SEND_DROPS.load(Ordering::Relaxed),
        udp_send_latency_p50_ticks: latency.udp_send_p50,
        udp_send_latency_p95_ticks: latency.udp_send_p95,
        udp_send_latency_p99_ticks: latency.udp_send_p99,
        udp_recv_latency_p50_ticks: latency.udp_recv_p50,
        udp_recv_latency_p95_ticks: latency.udp_recv_p95,
        udp_recv_latency_p99_ticks: latency.udp_recv_p99,
        tcp_send_latency_p50_ticks: latency.tcp_send_p50,
        tcp_send_latency_p95_ticks: latency.tcp_send_p95,
        tcp_send_latency_p99_ticks: latency.tcp_send_p99,
        tcp_recv_latency_p50_ticks: latency.tcp_recv_p50,
        tcp_recv_latency_p95_ticks: latency.tcp_recv_p95,
        tcp_recv_latency_p99_ticks: latency.tcp_recv_p99,
        tcp_recv_calls: latency.tcp_recv_calls,
        tcp_recv_hits: TCP_RECV_HITS.load(Ordering::Relaxed),
        tcp_queue_high_water: TCP_QUEUE_HIGH_WATER.load(Ordering::Relaxed),
        dns_register_calls: DNS_REGISTER_CALLS.load(Ordering::Relaxed),
        dns_resolve_calls: DNS_RESOLVE_CALLS.load(Ordering::Relaxed),
        dns_resolve_hits: DNS_RESOLVE_HITS.load(Ordering::Relaxed),
        filter_register_calls: FILTER_REGISTER_CALLS.load(Ordering::Relaxed),
        filter_remove_calls: FILTER_REMOVE_CALLS.load(Ordering::Relaxed),
        filter_clear_calls: FILTER_CLEAR_CALLS.load(Ordering::Relaxed),
        filter_eval_calls: FILTER_EVAL_CALLS.load(Ordering::Relaxed),
        filter_eval_allow: FILTER_EVAL_ALLOW.load(Ordering::Relaxed),
        filter_eval_drop: FILTER_EVAL_DROP.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_add_peer_calls: WG_ADD_PEER_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_remove_peer_calls: WG_REMOVE_PEER_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_encap_calls: WG_ENCAP_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_decap_calls: WG_DECAP_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_drop_calls: WG_DROP_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_bytes_encap: WG_BYTES_ENCAP.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_bytes_decap: WG_BYTES_DECAP.load(Ordering::Relaxed),
        #[cfg(feature = "network_wireguard")]
        wg_active_peers_high_water: WG_ACTIVE_PEERS_HIGH_WATER.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_register_calls: HTTP_REGISTER_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_remove_calls: HTTP_REMOVE_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_request_calls: HTTP_REQUEST_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_resp_200: HTTP_RESP_200.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_resp_304: HTTP_RESP_304.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_resp_404: HTTP_RESP_404.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_sendfile_calls: HTTP_SENDFILE_CALLS.load(Ordering::Relaxed),
        #[cfg(feature = "network_http")]
        http_bytes_served: HTTP_BYTES_SERVED.load(Ordering::Relaxed),
    }
}