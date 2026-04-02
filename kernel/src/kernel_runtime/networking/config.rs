#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) const NETWORK_IO_REBIND_STREAK_THRESHOLD: u64 = 3;
#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) const NETWORK_IO_FAILOVER_STREAK_THRESHOLD: u64 = 2;

#[cfg(all(feature = "drivers", feature = "networking"))]
#[inline(always)]
pub(super) fn network_slo_sample_interval() -> u64 {
    aethercore::config::KernelConfig::network_slo_sample_interval()
}

#[cfg(all(feature = "drivers", feature = "networking"))]
#[inline(always)]
pub(super) fn network_slo_log_interval_multiplier() -> u64 {
    aethercore::config::KernelConfig::network_slo_log_interval_multiplier()
}

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) const NETWORK_DRIVER_QUARANTINE_REBIND_FAILURES: u64 =
    if aethercore::generated_consts::DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES == 0 {
        1
    } else {
        aethercore::generated_consts::DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES
    };

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) const NETWORK_DRIVER_QUARANTINE_COOLDOWN_SAMPLES: u64 =
    if aethercore::generated_consts::DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES == 0 {
        1
    } else {
        aethercore::generated_consts::DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES
    };
