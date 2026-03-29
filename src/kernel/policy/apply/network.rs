#[inline(always)]
pub(super) fn apply_low_latency_network_policy() {
    #[cfg(feature = "libnet")]
    {
        let _ = crate::modules::libnet::apply_poll_profile(
            crate::modules::libnet::PollProfile::LowLatency,
        );
    }
    #[cfg(all(feature = "drivers", feature = "networking"))]
    {
        crate::modules::drivers::apply_network_poll_profile(
            crate::modules::drivers::NetworkPollProfile::LowLatency,
        );
        crate::modules::drivers::set_network_remediation_profile(
            crate::modules::drivers::NetworkRemediationProfile::Balanced,
        );
    }
}

#[inline(always)]
pub(super) fn apply_throughput_network_policy() {
    #[cfg(feature = "libnet")]
    {
        let _ = crate::modules::libnet::apply_poll_profile(
            crate::modules::libnet::PollProfile::Throughput,
        );
    }
    #[cfg(all(feature = "drivers", feature = "networking"))]
    {
        crate::modules::drivers::apply_network_poll_profile(
            crate::modules::drivers::NetworkPollProfile::Throughput,
        );
        crate::modules::drivers::set_network_remediation_profile(
            crate::modules::drivers::NetworkRemediationProfile::Conservative,
        );
    }
}

#[inline(always)]
pub(super) fn apply_aggressive_low_latency_network_policy() {
    #[cfg(feature = "libnet")]
    {
        let _ = crate::modules::libnet::apply_poll_profile(
            crate::modules::libnet::PollProfile::LowLatency,
        );
    }
    #[cfg(all(feature = "drivers", feature = "networking"))]
    {
        crate::modules::drivers::apply_network_poll_profile(
            crate::modules::drivers::NetworkPollProfile::LowLatency,
        );
        crate::modules::drivers::set_network_remediation_profile(
            crate::modules::drivers::NetworkRemediationProfile::Aggressive,
        );
    }
}
