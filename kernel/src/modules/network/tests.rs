use super::*;
use alloc::vec;
use crate::modules::network::metrics_ops::update_loopback_high_water;
#[cfg(feature = "network_transport")]
use crate::modules::network::filter_support::apply_filters;

#[test_case]
fn runtime_polling_toggle_roundtrip() {
    set_runtime_polling_enabled(false);
    assert!(!runtime_polling_enabled());
    set_runtime_polling_enabled(true);
    assert!(runtime_polling_enabled());
}

#[test_case]
fn runtime_poll_interval_clamps_to_one() {
    set_runtime_poll_interval_ticks(0);
    assert_eq!(
        runtime_poll_interval_ticks(),
        crate::config::KernelConfig::network_runtime_poll_interval_min()
    );
    set_runtime_poll_interval_ticks(4);
    assert_eq!(runtime_poll_interval_ticks(), 4);
}

#[test_case]
fn reset_runtime_stats_resets_interval_and_force_poll_counter() {
    set_runtime_poll_interval_ticks(7);
    let _ = force_poll_once();
    reset_runtime_stats();
    let stats = bridge_stats();
    assert_eq!(
        stats.smoltcp_poll_interval_ticks,
        crate::config::KernelConfig::network_runtime_poll_interval_min()
    );
    assert_eq!(stats.smoltcp_force_polls, 0);
}

#[test_case]
fn runtime_health_report_is_accessible_after_reset() {
    reset_runtime_stats();
    let report = runtime_health_report();
    assert_eq!(report.polls, 0);
    assert!(report.low_poll_activity);
    assert_eq!(
        recommended_runtime_health_action(),
        NetworkRuntimeHealthAction::ForcePollingUntilRecovered
    );
}

#[test_case]
fn loopback_reports_queue_drop_when_full() {
    reset_runtime_stats();
    let mut loopback = Loopback::new();
    for _ in 0..crate::config::KernelConfig::network_loopback_queue_limit() {
        let packet = Packet {
            data: vec![1, 2, 3, 4],
        };
        assert!(loopback.send(packet).is_ok());
    }
    let overflow = Packet {
        data: vec![9, 9, 9],
    };
    assert!(loopback.send(overflow).is_err());

    let stats = bridge_stats();
    assert_eq!(stats.loopback_send_drops, 1);
}

#[test_case]
fn loopback_backpressure_defer_policy_reports_telemetry() {
    reset_runtime_stats();
    set_backpressure_policy_table(BackpressurePolicyTable {
        loopback: BackpressurePolicy::Defer,
        #[cfg(feature = "network_transport")]
        udp: BackpressurePolicy::Drop,
        #[cfg(feature = "network_transport")]
        tcp: BackpressurePolicy::Drop,
    });

    let mut loopback = Loopback::new();
    for _ in 0..crate::config::KernelConfig::network_loopback_queue_limit() {
        let packet = Packet {
            data: vec![1, 2, 3, 4],
        };
        assert!(loopback.send(packet).is_ok());
    }
    let overflow = Packet {
        data: vec![9, 9, 9],
    };
    assert!(loopback.send(overflow).is_err());

    let stats = bridge_stats();
    assert!(stats.backpressure_defer_actions >= 1);
    assert_eq!(stats.loopback_backpressure_policy, 1);
}

#[cfg(feature = "network_transport")]
#[test_case]
fn network_alert_thresholds_report_drop_breach() {
    reset_runtime_stats();
    set_backpressure_policy_table(BackpressurePolicyTable {
        loopback: BackpressurePolicy::Drop,
        udp: BackpressurePolicy::Drop,
        tcp: BackpressurePolicy::Drop,
    });

    let sender = udp_bind(12000).expect("bind sender");
    let receiver = udp_bind(12001).expect("bind receiver");
    for _ in 0..crate::config::KernelConfig::network_udp_queue_limit() {
        assert!(sender.send_to(receiver.local_port(), b"x").is_ok());
    }
    assert!(sender.send_to(receiver.local_port(), b"overflow").is_err());

    set_network_alert_thresholds(NetworkAlertThresholds {
        min_health_score: 0,
        max_drops: 0,
        max_queue_high_water: u64::MAX,
    });
    let report = evaluate_network_alerts();
    assert!(report.drops_breach);
    assert!(report.breach_count >= 1);
}

#[cfg(feature = "network_transport")]
#[test_case]
fn udp_bind_send_recv_roundtrip() {
    reset_runtime_stats();
    let sender = udp_bind(10000).expect("bind sender");
    let receiver = udp_bind(10001).expect("bind receiver");

    assert_eq!(sender.send_to(receiver.local_port(), b"ping"), Ok(4));
    let msg = receiver.recv();
    assert!(msg.is_some());
    let msg = msg.unwrap_or(UdpDatagram {
        src_port: 0,
        dst_port: 0,
        payload: Vec::new(),
    });
    assert_eq!(msg.src_port, sender.local_port());
    assert_eq!(msg.dst_port, receiver.local_port());
    assert_eq!(msg.payload, b"ping");
}

#[cfg(feature = "network_transport")]
#[test_case]
fn udp_backpressure_force_poll_policy_reports_telemetry() {
    reset_runtime_stats();
    set_backpressure_policy_table(BackpressurePolicyTable {
        loopback: BackpressurePolicy::Drop,
        udp: BackpressurePolicy::ForcePoll,
        tcp: BackpressurePolicy::Drop,
    });

    let sender = udp_bind(11000).expect("bind sender");
    let receiver = udp_bind(11001).expect("bind receiver");

    for _ in 0..crate::config::KernelConfig::network_udp_queue_limit() {
        assert!(sender.send_to(receiver.local_port(), b"x").is_ok());
    }

    let overflow = sender.send_to(receiver.local_port(), b"overflow");
    assert!(overflow.is_err());

    let stats = bridge_stats();
    assert!(stats.backpressure_force_poll_actions >= 1);
    assert_eq!(stats.udp_backpressure_policy, 2);
}

#[cfg(feature = "network_transport")]
#[test_case]
fn tcp_listen_connect_send_recv_roundtrip() {
    reset_runtime_stats();
    let listener = tcp_listen(20001).expect("listen");
    let client = tcp_connect(20000, listener.local_port()).expect("connect");
    let server = listener.accept();
    assert!(server.is_some());
    let server = server.unwrap_or(TcpStream {
        local_port: 0,
        peer_port: 0,
    });

    assert_eq!(client.send(b"hello"), Ok(5));
    let got = server.recv();
    assert!(got.is_some());
    assert_eq!(got.unwrap_or_else(Vec::new), b"hello");
}

#[cfg(feature = "network_transport")]
#[test_case]
fn transport_latency_percentiles_are_monotonic() {
    reset_runtime_stats();

    let sender = udp_bind(21000).expect("bind sender");
    let receiver = udp_bind(21001).expect("bind receiver");
    for _ in 0..8 {
        let _ = sender.send_to(receiver.local_port(), b"latency");
        let _ = receiver.recv();
    }

    let listener = tcp_listen(22001).expect("listen");
    let client = tcp_connect(22000, listener.local_port()).expect("connect");
    let server = listener.accept().expect("accept");
    for _ in 0..8 {
        let _ = client.send(b"ticks");
        let _ = server.recv();
    }

    let stats = bridge_stats();
    assert!(stats.udp_send_latency_p50_ticks <= stats.udp_send_latency_p95_ticks);
    assert!(stats.udp_send_latency_p95_ticks <= stats.udp_send_latency_p99_ticks);
    assert!(stats.udp_recv_latency_p50_ticks <= stats.udp_recv_latency_p95_ticks);
    assert!(stats.udp_recv_latency_p95_ticks <= stats.udp_recv_latency_p99_ticks);
    assert!(stats.tcp_send_latency_p50_ticks <= stats.tcp_send_latency_p95_ticks);
    assert!(stats.tcp_send_latency_p95_ticks <= stats.tcp_send_latency_p99_ticks);
    assert!(stats.tcp_recv_latency_p50_ticks <= stats.tcp_recv_latency_p95_ticks);
    assert!(stats.tcp_recv_latency_p95_ticks <= stats.tcp_recv_latency_p99_ticks);
}

#[cfg(feature = "network_transport")]
#[test_case]
fn dns_register_and_resolve_roundtrip() {
    reset_runtime_stats();
    assert!(dns_register("core.local", [10, 0, 2, 15]).is_ok());
    assert_eq!(dns_resolve("core.local"), Some([10, 0, 2, 15]));
    assert_eq!(dns_resolve("missing.local"), None);
}

#[cfg(feature = "network_transport")]
#[test_case]
fn packet_filter_drops_udp_on_matching_rule() {
    reset_runtime_stats();
    let sender = udp_bind(30000).expect("bind sender");
    let receiver = udp_bind(30001).expect("bind receiver");

    let filter_id = register_packet_filter(
        FilterProtocol::Udp,
        Some(sender.local_port()),
        Some(receiver.local_port()),
        None,
        FilterAction::Drop,
    )
    .expect("register filter");

    assert!(sender.send_to(receiver.local_port(), b"blocked").is_err());
    assert!(receiver.recv().is_none());
    assert!(remove_packet_filter(filter_id));

    assert_eq!(sender.send_to(receiver.local_port(), b"allowed"), Ok(7));
    let packet = receiver.recv();
    assert!(packet.is_some());

    let stats = bridge_stats();
    assert!(stats.filter_eval_drop >= 1);
    assert!(stats.filter_remove_calls >= 1);
}

#[cfg(feature = "network_http")]
#[test_case]
fn http_sendfile_and_conditional_get_baseline() {
    reset_runtime_stats();
    assert!(http_register_static_asset("/index.html", "text/html", b"hello".to_vec()).is_ok());
    assert_eq!(http_static_asset_count(), 1);

    let view = http_sendfile("/index.html", 1, Some(3));
    assert!(view.is_some());
    let view = view.unwrap_or(HttpSendfileView {
        body: Arc::new(Vec::new()),
        offset: 0,
        len: 0,
    });
    assert_eq!(view.offset, 1);
    assert_eq!(view.len, 3);

    let ok = http_handle_static_request("GET", "/index.html", None);
    assert_eq!(ok.status, 200);
    assert!(ok.body.is_some());
    let etag = ok
        .headers
        .iter()
        .find(|(k, _)| k == "etag")
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .unwrap_or(0);
    assert!(etag != 0);

    let not_modified = http_handle_static_request("GET", "/index.html", Some(etag));
    assert_eq!(not_modified.status, 304);
    assert!(not_modified.body.is_none());

    let missing = http_handle_static_request("GET", "/missing", None);
    assert_eq!(missing.status, 404);

    let stats = bridge_stats();
    assert!(stats.http_register_calls >= 1);
    assert!(stats.http_sendfile_calls >= 1);
    assert!(stats.http_resp_200 >= 1);
    assert!(stats.http_resp_304 >= 1);
    assert!(stats.http_resp_404 >= 1);
    assert!(stats.http_bytes_served >= 5);
}

#[cfg(feature = "network_wireguard")]
#[test_case]
fn wireguard_peer_and_encap_decap_roundtrip() {
    reset_runtime_stats();
    let peer = wireguard_add_peer([7u8; 32], [10, 0, 2, 99], 51820).expect("add peer");
    assert_eq!(wireguard_peer_count(), 1);

    let packet = wireguard_encapsulate(peer, b"hello-wg").expect("encap");
    let decap = wireguard_decapsulate(&packet).expect("decap");
    assert_eq!(decap.0, peer);
    assert_eq!(decap.1, b"hello-wg");

    assert!(wireguard_remove_peer(peer));
    assert_eq!(wireguard_peer_count(), 0);

    let stats = bridge_stats();
    assert!(stats.wg_add_peer_calls >= 1);
    assert!(stats.wg_encap_calls >= 1);
    assert!(stats.wg_decap_calls >= 1);
}
// Example: Loopback Interface
pub struct Loopback {
    queue: Vec<Packet>,
}

impl Loopback {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }
}

impl NetworkInterface for Loopback {
    fn send(&mut self, packet: Packet) -> Result<(), &'static str> {
        LOOPBACK_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "network_transport")]
        apply_filters(FilterProtocol::Raw, None, None, packet.data.len())?;
        if self.queue.len() >= crate::config::KernelConfig::network_loopback_queue_limit() {
            match policy_from_u64(LOOPBACK_BACKPRESSURE_POLICY.load(Ordering::Relaxed)) {
                BackpressurePolicy::Drop => {
                    BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    LOOPBACK_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                    return Err("loopback queue full");
                }
                BackpressurePolicy::Defer => {
                    BACKPRESSURE_DEFER_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    return Err("loopback queue deferred");
                }
                BackpressurePolicy::ForcePoll => {
                    BACKPRESSURE_FORCE_POLL_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    let _ = force_poll_once();
                    if self.queue.len()
                        >= crate::config::KernelConfig::network_loopback_queue_limit()
                    {
                        BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                        LOOPBACK_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                        return Err("loopback queue full after force poll");
                    }
                }
            }
        }
        self.queue.push(packet);
        update_loopback_high_water(self.queue.len());
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<Packet>, &'static str> {
        LOOPBACK_RECEIVE_CALLS.fetch_add(1, Ordering::Relaxed);
        let packet = self.queue.pop();
        if packet.is_some() {
            LOOPBACK_RECEIVE_HITS.fetch_add(1, Ordering::Relaxed);
        }
        Ok(packet)
    }

    fn mac(&self) -> MacAddress {
        MacAddress::Ethernet([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
    }
}
