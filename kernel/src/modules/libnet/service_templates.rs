#[cfg(feature = "network_transport")]
use crate::modules::libnet::{DatagramSocket, StreamSocket};

#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceRunReport {
    pub sessions: usize,
    pub units_processed: usize,
    pub bytes_processed: usize,
    pub pump: crate::modules::libnet::LibNetPumpReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicePreset {
    ControlHeavy,
    ThroughputHeavy,
    PowerSave,
    LowLatency,
}

impl Default for ServicePreset {
    fn default() -> Self {
        Self::ControlHeavy
    }
}

pub fn recommended_service_preset() -> ServicePreset {
    preset_from_pressure_snapshot(crate::kernel::pressure::snapshot())
}

pub fn run_service_fast_path_cycle_auto() -> crate::modules::libnet::FastPathReport {
    run_service_fast_path_cycle(recommended_service_preset())
}

fn preset_from_pressure_snapshot(
    pressure: crate::kernel::pressure::CorePressureSnapshot,
) -> ServicePreset {
    if pressure.scheduler_class == crate::kernel::pressure::SchedulerPressureClass::Critical {
        if pressure.rt_starvation_alert {
            return ServicePreset::LowLatency;
        }
        return ServicePreset::ThroughputHeavy;
    }

    match pressure.class {
        crate::kernel::pressure::CorePressureClass::Critical => ServicePreset::ThroughputHeavy,
        crate::kernel::pressure::CorePressureClass::High => {
            if pressure.rt_starvation_alert {
                ServicePreset::LowLatency
            } else {
                ServicePreset::ThroughputHeavy
            }
        }
        crate::kernel::pressure::CorePressureClass::Elevated => ServicePreset::ControlHeavy,
        crate::kernel::pressure::CorePressureClass::Nominal => {
            if pressure.runqueue_total == 0 && pressure.net_saturation_percent < 20 {
                ServicePreset::PowerSave
            } else {
                ServicePreset::ControlHeavy
            }
        }
    }
}

pub fn preset_fast_path_config(preset: ServicePreset) -> crate::modules::libnet::FastPathConfig {
    let base_budget = crate::modules::libnet::l2::configured_default_pump_budget().max(1);

    match preset {
        ServicePreset::ControlHeavy => crate::modules::libnet::FastPathConfig {
            poll_strategy: crate::modules::libnet::PollStrategy::Fixed(
                crate::modules::libnet::PollProfile::Balanced,
            ),
            run_pump: true,
            collect_transport_snapshot: true,
            l2_pump_budget: core::cmp::max(1, base_budget / 2),
        },
        ServicePreset::ThroughputHeavy => crate::modules::libnet::FastPathConfig {
            poll_strategy: crate::modules::libnet::PollStrategy::Fixed(
                crate::modules::libnet::PollProfile::Throughput,
            ),
            run_pump: true,
            collect_transport_snapshot: false,
            l2_pump_budget: base_budget,
        },
        ServicePreset::PowerSave => crate::modules::libnet::FastPathConfig {
            poll_strategy: crate::modules::libnet::PollStrategy::Fixed(
                crate::modules::libnet::PollProfile::PowerSave,
            ),
            run_pump: true,
            collect_transport_snapshot: false,
            l2_pump_budget: core::cmp::max(1, base_budget / 4),
        },
        ServicePreset::LowLatency => crate::modules::libnet::FastPathConfig {
            poll_strategy: crate::modules::libnet::PollStrategy::Fixed(
                crate::modules::libnet::PollProfile::LowLatency,
            ),
            run_pump: true,
            collect_transport_snapshot: true,
            l2_pump_budget: core::cmp::max(1, base_budget / 2),
        },
    }
}

pub fn run_service_fast_path_cycle(
    preset: ServicePreset,
) -> crate::modules::libnet::FastPathReport {
    crate::modules::libnet::run_fast_path_once(preset_fast_path_config(preset))
}

#[cfg(feature = "network_transport")]
pub fn run_udp_relay_cycle(
    socket: &crate::modules::libnet::LibUdpSocket,
    upstream_port: u16,
    max_packets: usize,
) -> ServiceRunReport {
    let mut report = ServiceRunReport::default();
    let packets = socket.recv_batch(max_packets);
    report.units_processed = packets.len();

    for datagram in packets {
        if socket.send_to(upstream_port, &datagram.payload).is_ok() {
            report.bytes_processed += datagram.payload.len();
        }
    }

    report.pump = crate::modules::libnet::pump_once_with_report();
    report
}

#[cfg(feature = "network_transport")]
pub fn run_udp_relay_cycle_with_preset(
    socket: &crate::modules::libnet::LibUdpSocket,
    upstream_port: u16,
    max_packets: usize,
    preset: ServicePreset,
) -> ServiceRunReport {
    let mut report = run_udp_relay_cycle(socket, upstream_port, max_packets);
    report.pump = crate::modules::libnet::run_fast_path_once(preset_fast_path_config(preset)).pump;
    report
}

#[cfg(feature = "network_transport")]
pub fn run_tcp_echo_cycle(
    listener: &crate::modules::libnet::LibTcpListener,
    max_accepts: usize,
    max_chunks_per_stream: usize,
) -> ServiceRunReport {
    let mut report = ServiceRunReport::default();

    for _ in 0..max_accepts {
        let Some(stream) = listener.accept() else {
            break;
        };
        report.sessions += 1;

        let chunks = stream.recv_batch(max_chunks_per_stream);
        for chunk in chunks {
            report.units_processed += 1;
            report.bytes_processed += chunk.len();
            let _ = stream.send(&chunk);
        }
    }

    report.pump = crate::modules::libnet::pump_once_with_report();
    report
}

#[cfg(feature = "network_transport")]
pub fn run_tcp_echo_cycle_with_preset(
    listener: &crate::modules::libnet::LibTcpListener,
    max_accepts: usize,
    max_chunks_per_stream: usize,
    preset: ServicePreset,
) -> ServiceRunReport {
    let mut report = run_tcp_echo_cycle(listener, max_accepts, max_chunks_per_stream);
    report.pump = crate::modules::libnet::run_fast_path_once(preset_fast_path_config(preset)).pump;
    report
}

#[cfg(feature = "network_http")]
pub fn run_http_static_cycle(
    method: &str,
    path: &str,
    if_none_match: Option<u64>,
) -> crate::modules::libnet::HttpResponse {
    crate::modules::libnet::handle_static_request(method, path, if_none_match)
}

#[cfg(feature = "network_https")]
pub fn run_https_terminate_cycle(
    record: &[u8],
    out: &mut [u8],
) -> Result<(usize, crate::modules::libnet::LibNetPumpReport), &'static str> {
    let bytes = crate::modules::libnet::terminate_tls_record(record, out)?;
    let pump = crate::modules::libnet::pump_once_with_report();
    Ok((bytes, pump))
}

#[cfg(feature = "network_https")]
pub fn run_https_terminate_cycle_with_preset(
    record: &[u8],
    out: &mut [u8],
    preset: ServicePreset,
) -> Result<(usize, crate::modules::libnet::LibNetPumpReport), &'static str> {
    let bytes = crate::modules::libnet::terminate_tls_record(record, out)?;
    let pump = crate::modules::libnet::run_fast_path_once(preset_fast_path_config(preset)).pump;
    Ok((bytes, pump))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::pressure::{CorePressureClass, CorePressureSnapshot};

    #[cfg(feature = "network_transport")]
    use crate::modules::libnet::{DatagramSocket, StreamSocket};

    #[test_case]
    fn service_presets_produce_nonzero_budget() {
        let control = preset_fast_path_config(ServicePreset::ControlHeavy);
        let throughput = preset_fast_path_config(ServicePreset::ThroughputHeavy);
        let powersave = preset_fast_path_config(ServicePreset::PowerSave);
        let latency = preset_fast_path_config(ServicePreset::LowLatency);

        assert!(control.l2_pump_budget > 0);
        assert!(throughput.l2_pump_budget > 0);
        assert!(powersave.l2_pump_budget > 0);
        assert!(latency.l2_pump_budget > 0);
    }

    #[test_case]
    fn pressure_to_preset_mapping_prefers_throughput_on_critical() {
        let pressure = CorePressureSnapshot {
            schema_version: crate::kernel::pressure::CORE_PRESSURE_SCHEMA_VERSION,
            online_cpus: 4,
            runqueue_total: 24,
            runqueue_max: 16,
            runqueue_avg_milli: 6000,
            rt_starvation_alert: false,
            rt_forced_reschedules: 10,
            watchdog_stall_detections: 1,
            net_queue_limit: 1024,
            net_rx_depth: 900,
            net_tx_depth: 800,
            net_saturation_percent: 87,
            lb_imbalance_p50: 4,
            lb_imbalance_p90: 8,
            lb_imbalance_p99: 16,
            lb_prefer_local_forced_moves: 0,
            class: CorePressureClass::Critical,
            scheduler_class: crate::kernel::pressure::SchedulerPressureClass::Critical,
        };

        assert_eq!(
            preset_from_pressure_snapshot(pressure),
            ServicePreset::ThroughputHeavy
        );
    }

    #[test_case]
    fn pressure_to_preset_mapping_prefers_power_save_when_idle() {
        let pressure = CorePressureSnapshot {
            schema_version: crate::kernel::pressure::CORE_PRESSURE_SCHEMA_VERSION,
            online_cpus: 2,
            runqueue_total: 0,
            runqueue_max: 0,
            runqueue_avg_milli: 0,
            rt_starvation_alert: false,
            rt_forced_reschedules: 0,
            watchdog_stall_detections: 0,
            net_queue_limit: 1024,
            net_rx_depth: 0,
            net_tx_depth: 0,
            net_saturation_percent: 0,
            lb_imbalance_p50: 0,
            lb_imbalance_p90: 0,
            lb_imbalance_p99: 0,
            lb_prefer_local_forced_moves: 0,
            class: CorePressureClass::Nominal,
            scheduler_class: crate::kernel::pressure::SchedulerPressureClass::Nominal,
        };

        assert_eq!(
            preset_from_pressure_snapshot(pressure),
            ServicePreset::PowerSave
        );
    }

    #[test_case]
    fn scheduler_pressure_critical_overrides_nominal_core_class() {
        let pressure = CorePressureSnapshot {
            schema_version: crate::kernel::pressure::CORE_PRESSURE_SCHEMA_VERSION,
            online_cpus: 4,
            runqueue_total: 4,
            runqueue_max: 2,
            runqueue_avg_milli: 1000,
            rt_starvation_alert: false,
            rt_forced_reschedules: 0,
            watchdog_stall_detections: 0,
            net_queue_limit: 1024,
            net_rx_depth: 10,
            net_tx_depth: 8,
            net_saturation_percent: 1,
            lb_imbalance_p50: 4,
            lb_imbalance_p90: 8,
            lb_imbalance_p99: 24,
            lb_prefer_local_forced_moves: 2,
            class: CorePressureClass::Nominal,
            scheduler_class: crate::kernel::pressure::SchedulerPressureClass::Critical,
        };

        assert_eq!(
            preset_from_pressure_snapshot(pressure),
            ServicePreset::ThroughputHeavy
        );
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn udp_relay_cycle_processes_payloads_end_to_end() {
        let source = crate::modules::libnet::udp_bind(35100).expect("source bind");
        let relay = crate::modules::libnet::udp_bind(35101).expect("relay bind");
        let upstream = crate::modules::libnet::udp_bind(35102).expect("upstream bind");

        assert_eq!(source.send_to(relay.local_port(), b"relay-payload"), Ok(13));

        let report = run_udp_relay_cycle(&relay, upstream.local_port(), 8);
        assert!(report.units_processed >= 1);
        assert!(report.bytes_processed >= 13);

        let delivered = upstream.recv();
        assert!(delivered.is_some());
        assert_eq!(
            delivered
                .unwrap_or_else(|| panic!("missing datagram"))
                .payload,
            b"relay-payload"
        );
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn tcp_echo_cycle_processes_stream_chunks_end_to_end() {
        let listener = crate::modules::libnet::tcp_listen(35201).expect("listen");
        let client =
            crate::modules::libnet::tcp_connect(35200, listener.local_port()).expect("connect");

        assert_eq!(client.send(b"echo-line"), Ok(9));

        let report = run_tcp_echo_cycle(&listener, 1, 8);
        assert!(report.sessions >= 1);
        assert!(report.units_processed >= 1);
        assert!(report.bytes_processed >= 9);

        let echoed = client.recv();
        assert!(echoed.is_some());
        assert_eq!(
            echoed.unwrap_or_else(|| panic!("missing echo")),
            b"echo-line"
        );
    }
}
