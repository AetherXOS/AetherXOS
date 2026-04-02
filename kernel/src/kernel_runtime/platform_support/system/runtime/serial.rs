pub(crate) fn log_serial_runtime() {
    let serial = aethercore::hal::serial::stats();
    aethercore::klog_info!(
        "Serial runtime: tx_bytes={} drops={} spin_loops={} timeouts={}",
        serial.tx_bytes,
        serial.tx_drops,
        serial.tx_spin_loops,
        serial.tx_timeouts
    );
}
