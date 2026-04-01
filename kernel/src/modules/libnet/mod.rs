#[cfg(all(feature = "libnet_l7_http2", not(feature = "network_http")))]
compile_error!("feature 'libnet_l7_http2' requires feature 'network_http'");

#[cfg(all(feature = "libnet_l6_tls", not(feature = "network_https")))]
compile_error!("feature 'libnet_l6_tls' requires feature 'network_https'");

pub mod api;
pub mod bench;
pub mod control;
pub mod fast_path;
pub mod l2;
pub mod l34;
pub mod l6;
pub mod l7;
pub mod policy;
pub mod posix;
pub mod service_templates;

pub use api::LibNetApi;
pub use bench::{bench_bridge_surface, bench_fast_path_surface, LibNetMicroBenchReport};
pub use control::{apply_adaptive_profile, apply_poll_profile, current_bridge_health, PollProfile};
pub use fast_path::{
    run_default_cycle, run_once as run_fast_path_once, FastPathConfig, FastPathReport, PollStrategy,
};
pub use service_templates::{
    preset_fast_path_config, recommended_service_preset, run_service_fast_path_cycle,
    run_service_fast_path_cycle_auto, ServicePreset, ServiceRunReport,
};

#[cfg(feature = "network_transport")]
pub use bench::bench_transport_surface;
#[cfg(feature = "network_transport")]
pub use service_templates::{
    run_tcp_echo_cycle, run_tcp_echo_cycle_with_preset, run_udp_relay_cycle,
    run_udp_relay_cycle_with_preset,
};

#[cfg(feature = "network_http")]
pub use service_templates::run_http_static_cycle;

#[cfg(feature = "network_https")]
pub use service_templates::{run_https_terminate_cycle, run_https_terminate_cycle_with_preset};

#[cfg(feature = "network_transport")]
pub use l34::{
    clear_packet_filters, dns_register, dns_resolve, packet_filter_rules, register_packet_filter,
    remove_packet_filter, tcp_connect, tcp_listen, transport_snapshot, udp_bind,
    CustomSocketFactory, DatagramSocket, DefaultSocketFactory, FilterAction, FilterProtocol,
    LibTcpListener, LibTcpStream, LibUdpSocket, PacketFilterRule, StreamSocket, TransportSnapshot,
    UdpDatagram,
};

#[cfg(feature = "network_transport")]
pub use posix::{
    accept as posix_accept, accept4 as posix_accept4, accept4_errno as posix_accept4_errno,
    accept_errno as posix_accept_errno, bind as posix_bind, bind_errno as posix_bind_errno,
    close as posix_close, close_errno as posix_close_errno, connect as posix_connect,
    connect_errno as posix_connect_errno, dup as posix_dup, dup2 as posix_dup2,
    dup2_errno as posix_dup2_errno, dup_errno as posix_dup_errno, fcntl as posix_fcntl,
    fcntl_errno as posix_fcntl_errno, fcntl_getfl as posix_fcntl_getfl,
    fcntl_getfl_errno as posix_fcntl_getfl_errno, fcntl_setfl as posix_fcntl_setfl,
    fcntl_setfl_errno as posix_fcntl_setfl_errno, getpeername as posix_getpeername,
    getpeername_errno as posix_getpeername_errno, getsockname as posix_getsockname,
    getsockname_errno as posix_getsockname_errno, getsockopt as posix_getsockopt,
    getsockopt_errno as posix_getsockopt_errno, ioctl as posix_ioctl,
    ioctl_errno as posix_ioctl_errno, listen as posix_listen, listen_errno as posix_listen_errno,
    map_errno as posix_map_errno, poll as posix_poll, poll_errno as posix_poll_errno,
    recv as posix_recv, recv_errno as posix_recv_errno, recv_with_flags as posix_recv_with_flags,
    recv_with_flags_errno as posix_recv_with_flags_errno, recvfrom as posix_recvfrom,
    recvfrom_errno as posix_recvfrom_errno, recvfrom_with_flags as posix_recvfrom_with_flags,
    recvfrom_with_flags_errno as posix_recvfrom_with_flags_errno, select as posix_select,
    select_errno as posix_select_errno, send as posix_send, send_errno as posix_send_errno,
    sendto as posix_sendto, sendto_errno as posix_sendto_errno,
    set_nonblocking as posix_set_nonblocking, set_socket_option as posix_set_socket_option,
    setsockopt as posix_setsockopt, setsockopt_errno as posix_setsockopt_errno,
    shutdown as posix_shutdown, shutdown_errno as posix_shutdown_errno, socket as posix_socket,
    socket_errno as posix_socket_errno, socket_options as posix_socket_options,
    AddressFamily as PosixAddressFamily, FcntlCmd as PosixFcntlCmd, PosixErrno, PosixFdFlags,
    PosixIoctlCmd, PosixMsgFlags, PosixPollEvents, PosixPollFd, PosixRecvFrom, PosixSelectResult,
    PosixSockOpt, PosixSockOptVal, PosixSocketOptions, ShutdownHow as PosixShutdownHow,
    SocketAddrV4 as PosixSocketAddrV4, SocketOption as PosixSocketOption,
    SocketType as PosixSocketType,
};

#[cfg(feature = "network_https")]
pub use l6::{install_tls_server_config, terminate_tls_record, tls_stats};

#[cfg(feature = "network_http")]
pub use l7::{
    handle_static_request, register_static_asset, remove_static_asset, sendfile,
    static_asset_count, HttpResponse, HttpSendfileView,
};

#[derive(Debug, Clone, Copy)]
pub struct LibNetLayerStatus {
    pub l2_enabled: bool,
    pub l34_enabled: bool,
    pub l6_enabled: bool,
    pub l7_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct LibNetCapabilities {
    pub libnet_enabled: bool,
    pub l2_enabled: bool,
    pub l34_enabled: bool,
    pub l6_enabled: bool,
    pub l7_enabled: bool,
    pub transport_available: bool,
    pub https_available: bool,
    pub http_available: bool,
    pub http2_available: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct LibNetProfileCompatibility {
    pub strict_optional_features: bool,
    pub l34_requested: bool,
    pub l34_transport_feature: bool,
    pub l34_compatible: bool,
    pub l6_requested: bool,
    pub l6_https_feature: bool,
    pub l6_compatible: bool,
    pub l7_requested: bool,
    pub l7_http_feature: bool,
    pub l7_http2_feature: bool,
    pub l7_compatible: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct LibNetBridgeSnapshot {
    pub policy_network_surface_enabled: bool,
    pub core_rx_depth: usize,
    pub core_tx_depth: usize,
    pub core_queue_limit: usize,
    pub runtime_ready: bool,
    pub runtime_poll_enabled: bool,
    pub runtime_poll_interval_ticks: u64,
    pub runtime_health_score: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LibNetPumpReport {
    pub l2_frames_ingested: usize,
    pub l34_polled: bool,
}

pub fn layer_status() -> LibNetLayerStatus {
    LibNetLayerStatus {
        l2_enabled: crate::config::KernelConfig::libnet_l2_enabled(),
        l34_enabled: crate::config::KernelConfig::libnet_l34_enabled(),
        l6_enabled: crate::config::KernelConfig::libnet_l6_enabled(),
        l7_enabled: crate::config::KernelConfig::libnet_l7_enabled(),
    }
}

pub fn capabilities() -> LibNetCapabilities {
    let layer = layer_status();
    LibNetCapabilities {
        libnet_enabled: cfg!(feature = "libnet"),
        l2_enabled: layer.l2_enabled,
        l34_enabled: layer.l34_enabled,
        l6_enabled: layer.l6_enabled,
        l7_enabled: layer.l7_enabled,
        transport_available: cfg!(feature = "network_transport"),
        https_available: cfg!(feature = "network_https"),
        http_available: cfg!(feature = "network_http"),
        http2_available: cfg!(feature = "libnet_l7_http2"),
    }
}

pub fn bridge_snapshot() -> LibNetBridgeSnapshot {
    let core = crate::kernel::net_core::stats();
    let net = crate::modules::network::bridge::stats();
    LibNetBridgeSnapshot {
        policy_network_surface_enabled: crate::config::KernelConfig::is_network_library_api_exposed(
        ),
        core_rx_depth: core.rx_depth,
        core_tx_depth: core.tx_depth,
        core_queue_limit: core.queue_limit,
        runtime_ready: net.smoltcp_runtime_ready,
        runtime_poll_enabled: net.smoltcp_runtime_poll_enabled,
        runtime_poll_interval_ticks: net.smoltcp_poll_interval_ticks,
        runtime_health_score: net.smoltcp_health_score,
    }
}

pub fn profile_compatibility() -> LibNetProfileCompatibility {
    let l34_requested = crate::config::KernelConfig::libnet_l34_enabled();
    let l34_transport_feature = cfg!(feature = "network_transport");

    let l6_requested = crate::config::KernelConfig::libnet_l6_enabled();
    let l6_https_feature = cfg!(feature = "network_https");

    let l7_requested = crate::config::KernelConfig::libnet_l7_enabled();
    let l7_http_feature = cfg!(feature = "network_http");
    let l7_http2_feature = cfg!(feature = "libnet_l7_http2");

    LibNetProfileCompatibility {
        strict_optional_features: crate::config::KernelConfig::is_strict_optional_features_enabled(
        ),
        l34_requested,
        l34_transport_feature,
        l34_compatible: !l34_requested || l34_transport_feature,
        l6_requested,
        l6_https_feature,
        l6_compatible: !l6_requested || l6_https_feature,
        l7_requested,
        l7_http_feature,
        l7_http2_feature,
        l7_compatible: !l7_requested || l7_http_feature || l7_http2_feature,
    }
}

pub fn pump_once_with_report() -> LibNetPumpReport {
    pump_once_with_budget_report(None)
}

pub fn pump_once_with_budget_report(l2_budget: Option<usize>) -> LibNetPumpReport {
    let mut report = LibNetPumpReport {
        l2_frames_ingested: 0,
        l34_polled: false,
    };

    if crate::config::KernelConfig::libnet_l2_enabled() {
        report.l2_frames_ingested = match l2_budget {
            Some(budget) => l2::pump_core_frames_into_libnet_with_budget(budget),
            None => l2::pump_core_frames_into_libnet(),
        };
    }
    if crate::config::KernelConfig::libnet_l34_enabled() {
        report.l34_polled = l34::poll_transport_once();
    }
    report
}

pub fn pump_once() {
    let _ = pump_once_with_report();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn capability_flags_align_with_features() {
        let caps = capabilities();
        assert_eq!(
            caps.transport_available,
            cfg!(feature = "network_transport")
        );
        assert_eq!(caps.https_available, cfg!(feature = "network_https"));
        assert_eq!(caps.http_available, cfg!(feature = "network_http"));
    }

    #[test_case]
    fn pump_report_returns_structured_result() {
        let report = pump_once_with_report();
        assert!(report.l2_frames_ingested <= crate::kernel::net_core::queue_limit());
    }

    #[test_case]
    fn pump_report_budget_override_is_bounded() {
        let report = pump_once_with_budget_report(Some(usize::MAX));
        assert!(report.l2_frames_ingested <= l2::core_to_libnet_batch_size());
    }
}
