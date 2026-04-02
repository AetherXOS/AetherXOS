use super::super::super::super::*;

pub(super) fn log_network_dataplane_dashboard() {
    let net = aethercore::modules::drivers::network_dataplane_stats();
    aethercore::klog_info!(
        "Network dataplane: active={:?} profile={:?} io_owned={} regs(v={},e={}) service(loop={},irq={}) tx(frames={},drops={}) rx(frames={},drops={}) rings(vrx={},vtx={},erx={},etx={}) e1000(io_calls={} rx={}/{} tx={}/{} trunc={} busy={} lock={} err={}) quarantine(v={},e={},events={},threshold={},cooldown={})",
        net.active_driver,
        net.poll_profile,
        net.driver_io_owned,
        net.register_virtio_calls,
        net.register_e1000_calls,
        net.service_calls,
        net.irq_service_calls,
        net.tx_to_nic_frames,
        net.tx_to_nic_drops,
        net.rx_to_core_frames,
        net.rx_to_core_drops,
        net.virtio_rx_depth,
        net.virtio_tx_depth,
        net.e1000_rx_depth,
        net.e1000_tx_depth,
        net.e1000_io_calls,
        net.e1000_rx_frames,
        net.e1000_rx_bytes,
        net.e1000_tx_frames,
        net.e1000_tx_bytes,
        net.e1000_tx_truncated_frames,
        net.e1000_tx_desc_busy_events,
        net.e1000_tx_lock_contention_events,
        net.e1000_io_errors,
        NETWORK_DRIVER_QUARANTINE_VIRTIO.load(core::sync::atomic::Ordering::Relaxed),
        NETWORK_DRIVER_QUARANTINE_E1000.load(core::sync::atomic::Ordering::Relaxed),
        NETWORK_DRIVER_QUARANTINE_EVENTS.load(core::sync::atomic::Ordering::Relaxed),
        NETWORK_DRIVER_QUARANTINE_REBIND_FAILURES,
        NETWORK_DRIVER_QUARANTINE_COOLDOWN_SAMPLES
    );
}
