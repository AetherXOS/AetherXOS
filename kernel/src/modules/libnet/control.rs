#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollProfile {
    LowLatency,
    Balanced,
    Throughput,
    PowerSave,
}

pub fn apply_poll_profile(profile: PollProfile) -> crate::modules::libnet::LibNetBridgeSnapshot {
    match profile {
        PollProfile::LowLatency => {
            crate::modules::libnet::l34::set_polling_enabled(true);
            crate::modules::libnet::l34::set_poll_interval_ticks(
                crate::config::KernelConfig::libnet_poll_interval_low_latency(),
            );
        }
        PollProfile::Balanced => {
            crate::modules::libnet::l34::set_polling_enabled(true);
            crate::modules::libnet::l34::set_poll_interval_ticks(
                crate::config::KernelConfig::libnet_poll_interval_balanced(),
            );
        }
        PollProfile::Throughput => {
            crate::modules::libnet::l34::set_polling_enabled(true);
            crate::modules::libnet::l34::set_poll_interval_ticks(
                crate::config::KernelConfig::libnet_poll_interval_low_latency(),
            );
        }
        PollProfile::PowerSave => {
            crate::modules::libnet::l34::set_polling_enabled(true);
            crate::modules::libnet::l34::set_poll_interval_ticks(
                crate::config::KernelConfig::libnet_poll_interval_powersave(),
            );
        }
    }
    crate::modules::libnet::bridge_snapshot()
}

pub fn current_bridge_health() -> u64 {
    crate::modules::libnet::bridge_snapshot().runtime_health_score
}

pub fn apply_adaptive_profile() -> crate::modules::libnet::LibNetBridgeSnapshot {
    let snapshot = crate::modules::libnet::bridge_snapshot();

    let queue_depth_divisor = crate::config::KernelConfig::libnet_adaptive_queue_depth_divisor();
    let health_low_threshold = crate::config::KernelConfig::libnet_adaptive_health_low_threshold();
    let poll_high_threshold = crate::config::KernelConfig::libnet_adaptive_poll_high_threshold();

    let selected = if snapshot.core_rx_depth > (snapshot.core_queue_limit / queue_depth_divisor) {
        PollProfile::Throughput
    } else if snapshot.runtime_health_score < health_low_threshold {
        PollProfile::LowLatency
    } else if snapshot.runtime_poll_interval_ticks > poll_high_threshold {
        PollProfile::Balanced
    } else {
        PollProfile::PowerSave
    };

    apply_poll_profile(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn poll_profile_matrix_sets_expected_runtime_intervals() {
        crate::config::KernelConfig::reset_runtime_overrides();
        let low_interval = crate::config::KernelConfig::libnet_poll_interval_low_latency();
        let balanced_interval = crate::config::KernelConfig::libnet_poll_interval_balanced();
        let powersave_interval = crate::config::KernelConfig::libnet_poll_interval_powersave();

        let low = apply_poll_profile(PollProfile::LowLatency);
        assert!(low.runtime_poll_enabled);
        assert_eq!(low.runtime_poll_interval_ticks, low_interval);

        let balanced = apply_poll_profile(PollProfile::Balanced);
        assert!(balanced.runtime_poll_enabled);
        assert_eq!(balanced.runtime_poll_interval_ticks, balanced_interval);

        let throughput = apply_poll_profile(PollProfile::Throughput);
        assert!(throughput.runtime_poll_enabled);
        assert_eq!(throughput.runtime_poll_interval_ticks, low_interval);

        let powersave = apply_poll_profile(PollProfile::PowerSave);
        assert!(powersave.runtime_poll_enabled);
        assert_eq!(powersave.runtime_poll_interval_ticks, powersave_interval);
    }

    #[test_case]
    fn adaptive_profile_returns_valid_interval_band() {
        crate::config::KernelConfig::reset_runtime_overrides();
        let low_interval = crate::config::KernelConfig::libnet_poll_interval_low_latency();
        let balanced_interval = crate::config::KernelConfig::libnet_poll_interval_balanced();
        let powersave_interval = crate::config::KernelConfig::libnet_poll_interval_powersave();

        let snap = apply_adaptive_profile();
        assert!(snap.runtime_poll_enabled);
        assert!(matches!(
            snap.runtime_poll_interval_ticks,
            value if value == low_interval || value == balanced_interval || value == powersave_interval
        ));
    }
}
