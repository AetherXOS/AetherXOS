use super::*;

#[test_case]
fn network_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::NetworkRuntimeProfile {
        tls_policy_profile: super::TlsPolicyProfile::Minimal,
        slo: super::NetworkSloRuntimeConfig {
            sample_interval: 33,
            log_interval_multiplier: 7,
        },
    };
    KernelConfig::set_network_runtime_profile(Some(profile));
    let got = KernelConfig::network_runtime_profile();
    assert_eq!(got.tls_policy_profile, super::TlsPolicyProfile::Minimal);
    assert_eq!(got.slo.sample_interval, 33);
    assert_eq!(got.slo.log_interval_multiplier, 7);

    KernelConfig::set_network_runtime_profile(None);
    let reset = KernelConfig::network_runtime_profile();
    assert_eq!(reset.tls_policy_profile, super::TlsPolicyProfile::Balanced);
    assert_eq!(
        reset.slo.sample_interval,
        crate::generated_consts::NETWORK_SLO_SAMPLE_INTERVAL
    );
    assert_eq!(
        reset.slo.log_interval_multiplier,
        crate::generated_consts::NETWORK_SLO_LOG_INTERVAL_MULTIPLIER
    );
}
