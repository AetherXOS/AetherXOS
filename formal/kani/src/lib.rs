#[cfg(kani)]
#[kani::proof]
fn telemetry_history_len_never_drops_below_one() {
    hypercore::config::KernelConfig::set_telemetry_history_len(Some(1));
    assert!(hypercore::config::KernelConfig::telemetry_history_len() >= 1);
    hypercore::config::KernelConfig::set_telemetry_history_len(None);
}
