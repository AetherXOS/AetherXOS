#[cfg(feature = "networking")]
pub(crate) fn log_network_transport_telemetry() {
    let net = hypercore::modules::network::bridge::stats();
    #[cfg(feature = "network_http")]
    hypercore::klog_info!(
        "Network transport: udp_bind={} udp_send={} udp_drop={} udp_recv={} udp_hits={} udp_high_water={} tcp_listen={} tcp_connect={} tcp_accept={}/{} tcp_send={} tcp_drop={} tcp_recv={}/{} tcp_high_water={} dns_register={} dns_resolve={}/{} filter_reg={} filter_rm={} filter_clear={} filter_eval={}/{} filter_drop={} http_reg={} http_rm={} http_req={} http_200={} http_304={} http_404={} http_sendfile={} http_bytes={} loopback_high_water={}",
        net.udp_bind_calls,
        net.udp_send_calls,
        net.udp_send_drops,
        net.udp_recv_calls,
        net.udp_recv_hits,
        net.udp_queue_high_water,
        net.tcp_listen_calls,
        net.tcp_connect_calls,
        net.tcp_accept_calls,
        net.tcp_accept_hits,
        net.tcp_send_calls,
        net.tcp_send_drops,
        net.tcp_recv_calls,
        net.tcp_recv_hits,
        net.tcp_queue_high_water,
        net.dns_register_calls,
        net.dns_resolve_calls,
        net.dns_resolve_hits,
        net.filter_register_calls,
        net.filter_remove_calls,
        net.filter_clear_calls,
        net.filter_eval_calls,
        net.filter_eval_allow,
        net.filter_eval_drop,
        net.http_register_calls,
        net.http_remove_calls,
        net.http_request_calls,
        net.http_resp_200,
        net.http_resp_304,
        net.http_resp_404,
        net.http_sendfile_calls,
        net.http_bytes_served,
        net.loopback_queue_high_water
    );

    #[cfg(not(feature = "network_http"))]
    hypercore::klog_info!(
        "Network transport: udp_bind={} udp_send={} udp_drop={} udp_recv={} udp_hits={} udp_high_water={} tcp_listen={} tcp_connect={} tcp_accept={}/{} tcp_send={} tcp_drop={} tcp_recv={}/{} tcp_high_water={} dns_register={} dns_resolve={}/{} filter_reg={} filter_rm={} filter_clear={} filter_eval={}/{} filter_drop={} loopback_high_water={}",
        net.udp_bind_calls,
        net.udp_send_calls,
        net.udp_send_drops,
        net.udp_recv_calls,
        net.udp_recv_hits,
        net.udp_queue_high_water,
        net.tcp_listen_calls,
        net.tcp_connect_calls,
        net.tcp_accept_calls,
        net.tcp_accept_hits,
        net.tcp_send_calls,
        net.tcp_send_drops,
        net.tcp_recv_calls,
        net.tcp_recv_hits,
        net.tcp_queue_high_water,
        net.dns_register_calls,
        net.dns_resolve_calls,
        net.dns_resolve_hits,
        net.filter_register_calls,
        net.filter_remove_calls,
        net.filter_clear_calls,
        net.filter_eval_calls,
        net.filter_eval_allow,
        net.filter_eval_drop,
        net.loopback_queue_high_water
    );

    #[cfg(feature = "network_wireguard")]
    hypercore::klog_info!(
        "Network wireguard: peers_high_water={} add={} remove={} encap={} decap={} drops={} bytes={}/{}",
        net.wg_active_peers_high_water,
        net.wg_add_peer_calls,
        net.wg_remove_peer_calls,
        net.wg_encap_calls,
        net.wg_decap_calls,
        net.wg_drop_calls,
        net.wg_bytes_encap,
        net.wg_bytes_decap
    );
}

#[cfg(feature = "networking")]
pub(crate) fn log_network_bridge_runtime() {
    let net = hypercore::modules::network::bridge::stats();
    hypercore::klog_info!(
        "Networking bridge: ready={} poll_enabled={} poll_interval={} inits={} polls={} force_polls={} reinit_calls={} rx={} tx={} init_err={} poll_err={} poll_skips={} control_updates={} loop_send={} loop_drop={} loop_recv={} loop_recv_hits={} health={}",
        net.smoltcp_runtime_ready,
        net.smoltcp_runtime_poll_enabled,
        net.smoltcp_poll_interval_ticks,
        net.smoltcp_bridge_inits,
        net.smoltcp_polls,
        net.smoltcp_force_polls,
        net.smoltcp_reinitialize_calls,
        net.smoltcp_rx_frames,
        net.smoltcp_tx_frames,
        net.smoltcp_init_errors,
        net.smoltcp_poll_errors,
        net.smoltcp_poll_skips,
        net.smoltcp_runtime_control_updates,
        net.loopback_send_calls,
        net.loopback_send_drops,
        net.loopback_receive_calls,
        net.loopback_receive_hits,
        net.smoltcp_health_score
    );
}

#[cfg(all(feature = "networking", feature = "libnet"))]
pub(crate) fn log_libnet_runtime() {
    let caps = hypercore::modules::libnet::capabilities();
    hypercore::klog_info!(
        "LibNet capabilities: enabled={} l2={} l34={} l6={} l7={} transport={} https={} http={} http2={}",
        caps.libnet_enabled,
        caps.l2_enabled,
        caps.l34_enabled,
        caps.l6_enabled,
        caps.l7_enabled,
        caps.transport_available,
        caps.https_available,
        caps.http_available,
        caps.http2_available
    );

    let compat = hypercore::modules::libnet::profile_compatibility();
    hypercore::klog_info!(
        "LibNet profile compatibility: strict={} l34(req={},feature={},ok={}) l6(req={},feature={},ok={}) l7(req={},http={},http2={},ok={})",
        compat.strict_optional_features,
        compat.l34_requested,
        compat.l34_transport_feature,
        compat.l34_compatible,
        compat.l6_requested,
        compat.l6_https_feature,
        compat.l6_compatible,
        compat.l7_requested,
        compat.l7_http_feature,
        compat.l7_http2_feature,
        compat.l7_compatible
    );

    let bridge = hypercore::modules::libnet::bridge_snapshot();
    hypercore::klog_info!(
        "LibNet bridge snapshot: policy_network={} core_rx_depth={} core_tx_depth={} core_queue_limit={} runtime_ready={} poll_enabled={} poll_interval={} health={}",
        bridge.policy_network_surface_enabled,
        bridge.core_rx_depth,
        bridge.core_tx_depth,
        bridge.core_queue_limit,
        bridge.runtime_ready,
        bridge.runtime_poll_enabled,
        bridge.runtime_poll_interval_ticks,
        bridge.runtime_health_score
    );
}

#[cfg(feature = "networking")]
pub(crate) fn init_network_bridge_runtime(
    telemetry: super::super::config::PlatformTelemetryConfig,
) {
    let nic = crate::kernel_runtime::networking::KernelLoopbackNic::new();
    if hypercore::modules::network::bridge::init_smoltcp_runtime(&nic).is_ok() {
        if telemetry.network_runtime() {
            log_network_bridge_runtime();

            #[cfg(feature = "libnet")]
            log_libnet_runtime();
        }
    } else {
        hypercore::klog_warn!("Networking bridge initialization failed");
    }
}
