#[cfg(all(feature = "drivers", feature = "networking"))]
mod config;
#[cfg(all(feature = "drivers", feature = "networking"))]
mod counters;
mod loopback;

#[cfg(feature = "networking")]
pub(crate) use loopback::KernelLoopbackNic;

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(crate) use config::{
    network_slo_log_interval_multiplier, network_slo_sample_interval,
    NETWORK_DRIVER_QUARANTINE_COOLDOWN_SAMPLES, NETWORK_DRIVER_QUARANTINE_REBIND_FAILURES,
    NETWORK_IO_FAILOVER_STREAK_THRESHOLD, NETWORK_IO_REBIND_STREAK_THRESHOLD,
};
#[cfg(all(feature = "drivers", feature = "networking"))]
pub(crate) use counters::{
    E1000_IO_ERROR_STREAK, E1000_REBIND_FAILURE_STREAK, NETWORK_AUTO_POLICY_SWITCH_COOLDOWN,
    NETWORK_AUTO_POLICY_SWITCH_COUNT, NETWORK_DRIVER_QUARANTINE_E1000,
    NETWORK_DRIVER_QUARANTINE_EVENTS, NETWORK_DRIVER_QUARANTINE_VIRTIO, NETWORK_SLO_BREACH_STREAK,
    NETWORK_SLO_LAST_LOG_SAMPLE, NETWORK_SLO_REMEDIATION_ACTIONS, NETWORK_SLO_REMEDIATION_STAGE,
    NETWORK_SLO_SAMPLE_COUNTER, VIRTIO_IO_ERROR_STREAK, VIRTIO_REBIND_FAILURE_STREAK,
};
