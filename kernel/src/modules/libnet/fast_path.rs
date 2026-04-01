#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollStrategy {
    Unchanged,
    Adaptive,
    Fixed(crate::modules::libnet::PollProfile),
}

impl Default for PollStrategy {
    fn default() -> Self {
        Self::Unchanged
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FastPathConfig {
    pub poll_strategy: PollStrategy,
    pub run_pump: bool,
    pub collect_transport_snapshot: bool,
    pub l2_pump_budget: usize,
}

impl Default for FastPathConfig {
    fn default() -> Self {
        default_from_kernel_config()
    }
}

fn strategy_from_kernel_config(value: crate::config::LibNetFastPathStrategy) -> PollStrategy {
    match value {
        crate::config::LibNetFastPathStrategy::Unchanged => PollStrategy::Unchanged,
        crate::config::LibNetFastPathStrategy::Adaptive => PollStrategy::Adaptive,
        crate::config::LibNetFastPathStrategy::LowLatency => {
            PollStrategy::Fixed(crate::modules::libnet::PollProfile::LowLatency)
        }
        crate::config::LibNetFastPathStrategy::Balanced => {
            PollStrategy::Fixed(crate::modules::libnet::PollProfile::Balanced)
        }
        crate::config::LibNetFastPathStrategy::Throughput => {
            PollStrategy::Fixed(crate::modules::libnet::PollProfile::Throughput)
        }
        crate::config::LibNetFastPathStrategy::PowerSave => {
            PollStrategy::Fixed(crate::modules::libnet::PollProfile::PowerSave)
        }
    }
}

pub fn default_from_kernel_config() -> FastPathConfig {
    FastPathConfig {
        poll_strategy: strategy_from_kernel_config(
            crate::config::KernelConfig::libnet_fast_path_strategy(),
        ),
        run_pump: crate::config::KernelConfig::libnet_fast_path_run_pump(),
        collect_transport_snapshot:
            crate::config::KernelConfig::libnet_fast_path_collect_transport_snapshot(),
        l2_pump_budget: crate::config::KernelConfig::libnet_fast_path_pump_budget(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FastPathReport {
    pub poll_strategy: PollStrategy,
    pub bridge_before: crate::modules::libnet::LibNetBridgeSnapshot,
    pub bridge_after: crate::modules::libnet::LibNetBridgeSnapshot,
    pub pump: crate::modules::libnet::LibNetPumpReport,
    #[cfg(feature = "network_transport")]
    pub transport: Option<crate::modules::libnet::TransportSnapshot>,
}

pub fn run_once(config: FastPathConfig) -> FastPathReport {
    let bridge_before = crate::modules::libnet::bridge_snapshot();

    match config.poll_strategy {
        PollStrategy::Unchanged => {}
        PollStrategy::Adaptive => {
            let _ = crate::modules::libnet::apply_adaptive_profile();
        }
        PollStrategy::Fixed(profile) => {
            let _ = crate::modules::libnet::apply_poll_profile(profile);
        }
    }

    let pump = if config.run_pump {
        crate::modules::libnet::pump_once_with_budget_report(Some(config.l2_pump_budget))
    } else {
        crate::modules::libnet::LibNetPumpReport {
            l2_frames_ingested: 0,
            l34_polled: false,
        }
    };

    let bridge_after = crate::modules::libnet::bridge_snapshot();

    FastPathReport {
        poll_strategy: config.poll_strategy,
        bridge_before,
        bridge_after,
        pump,
        #[cfg(feature = "network_transport")]
        transport: if config.collect_transport_snapshot {
            Some(crate::modules::libnet::transport_snapshot())
        } else {
            None
        },
    }
}

pub fn run_default_cycle() -> FastPathReport {
    run_once(FastPathConfig::default())
}
