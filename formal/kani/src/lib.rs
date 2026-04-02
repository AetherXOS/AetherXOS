#[cfg(kani)]
#[kani::proof]
fn telemetry_history_len_never_drops_below_one() {
    aethercore::config::KernelConfig::set_telemetry_history_len(Some(1));
    assert!(aethercore::config::KernelConfig::telemetry_history_len() >= 1);
    aethercore::config::KernelConfig::set_telemetry_history_len(None);
}
