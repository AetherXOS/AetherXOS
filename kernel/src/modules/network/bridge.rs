use alloc::vec::Vec;

pub use crate::modules::network::NetworkBridgeStats;
pub use crate::modules::network::{BackpressurePolicy, BackpressurePolicyTable};
pub use crate::modules::network::{NetworkAlertReport, NetworkAlertThresholds};

pub fn stats() -> NetworkBridgeStats {
    crate::modules::network::bridge_stats()
}

pub fn init_smoltcp_runtime(
    nic: &dyn crate::modules::network::NetworkInterface,
) -> Result<(), &'static str> {
    crate::modules::network::init_smoltcp_runtime(nic)
}

pub fn reinitialize_smoltcp_runtime(
    nic: &dyn crate::modules::network::NetworkInterface,
) -> Result<(), &'static str> {
    crate::modules::network::reinitialize_smoltcp_runtime(nic)
}

pub fn poll_smoltcp_runtime() -> bool {
    crate::modules::network::poll_smoltcp_runtime()
}

pub fn force_poll_once() -> bool {
    crate::modules::network::force_poll_once()
}

pub fn ingest_raw_ethernet_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    crate::modules::network::ingest_raw_ethernet_frame(frame)
}

pub fn ingest_raw_ethernet_frames(frames: Vec<Vec<u8>>) -> usize {
    crate::modules::network::ingest_raw_ethernet_frames(frames)
}

pub fn set_runtime_polling_enabled(enabled: bool) {
    crate::modules::network::set_runtime_polling_enabled(enabled)
}

pub fn set_runtime_poll_interval_ticks(interval: u64) {
    crate::modules::network::set_runtime_poll_interval_ticks(interval)
}

pub fn runtime_polling_enabled() -> bool {
    crate::modules::network::runtime_polling_enabled()
}

pub fn runtime_poll_interval_ticks() -> u64 {
    crate::modules::network::runtime_poll_interval_ticks()
}

pub fn reset_runtime_stats() {
    crate::modules::network::reset_runtime_stats()
}

pub fn backpressure_policy_table() -> BackpressurePolicyTable {
    crate::modules::network::backpressure_policy_table()
}

pub fn set_backpressure_policy_table(table: BackpressurePolicyTable) {
    crate::modules::network::set_backpressure_policy_table(table)
}

pub fn network_alert_thresholds() -> NetworkAlertThresholds {
    crate::modules::network::network_alert_thresholds()
}

pub fn set_network_alert_thresholds(thresholds: NetworkAlertThresholds) {
    crate::modules::network::set_network_alert_thresholds(thresholds)
}

pub fn evaluate_network_alerts() -> NetworkAlertReport {
    crate::modules::network::evaluate_network_alerts()
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test_case]
    fn bridge_runtime_controls_are_compatible_with_root_api() {
        reset_runtime_stats();

        set_runtime_polling_enabled(false);
        assert_eq!(
            runtime_polling_enabled(),
            crate::modules::network::runtime_polling_enabled()
        );

        set_runtime_poll_interval_ticks(7);
        assert_eq!(
            runtime_poll_interval_ticks(),
            crate::modules::network::runtime_poll_interval_ticks()
        );

        set_runtime_polling_enabled(true);
    }

    #[test_case]
    fn bridge_backpressure_and_alert_thresholds_match_root_api() {
        reset_runtime_stats();

        let mut table = backpressure_policy_table();
        table.loopback = BackpressurePolicy::ForcePoll;
        #[cfg(feature = "network_transport")]
        {
            table.udp = BackpressurePolicy::Defer;
            table.tcp = BackpressurePolicy::Drop;
        }
        set_backpressure_policy_table(table);

        let root_table = crate::modules::network::backpressure_policy_table();
        assert_eq!(backpressure_policy_table(), root_table);

        let thresholds = NetworkAlertThresholds {
            min_health_score: 50,
            max_drops: 3,
            max_queue_high_water: 2,
        };
        set_network_alert_thresholds(thresholds);
        assert_eq!(
            network_alert_thresholds(),
            crate::modules::network::network_alert_thresholds()
        );

        let bridge_report = evaluate_network_alerts();
        let root_report = crate::modules::network::evaluate_network_alerts();
        assert_eq!(bridge_report, root_report);
    }

    #[test_case]
    fn bridge_stress_test_queue_pressure() {
        reset_runtime_stats();
        let mut table = backpressure_policy_table();
        table.loopback = BackpressurePolicy::Drop;
        set_backpressure_policy_table(table);

        // Simulate high ingest rate
        let frames = vec![vec![0u8; 64]; 1000];
        let ingested = ingest_raw_ethernet_frames(frames);
        assert!(ingested > 0);

        let report = evaluate_network_alerts();
        assert!(
            report.health_breach
                || report.drops_breach
                || report.queue_breach
                || report.breach_count > 0
        );
    }
}
