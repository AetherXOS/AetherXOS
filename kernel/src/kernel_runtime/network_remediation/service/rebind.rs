use aethercore::modules::drivers::{
    ActiveNetworkDriver, DriverLifecycle, NetworkQueueClearStats, VirtIoNet, E1000,
};

pub(super) fn rebind_virtio_driver(runtime_driver: &mut VirtIoNet) -> bool {
    rebind_network_driver(
        runtime_driver,
        ActiveNetworkDriver::VirtIo,
        aethercore::modules::drivers::register_virtio_network_dataplane,
        |stats| (stats.cleared_virtio_rx, stats.cleared_virtio_tx),
        "VirtIO",
        "vrx",
        "vtx",
    )
}

pub(super) fn rebind_e1000_driver(runtime_driver: &mut E1000) -> bool {
    rebind_network_driver(
        runtime_driver,
        ActiveNetworkDriver::E1000,
        aethercore::modules::drivers::register_e1000_network_dataplane,
        |stats| (stats.cleared_e1000_rx, stats.cleared_e1000_tx),
        "E1000",
        "erx",
        "etx",
    )
}

fn rebind_network_driver<T: DriverLifecycle>(
    runtime_driver: &mut T,
    driver_kind: ActiveNetworkDriver,
    register_dataplane: fn(),
    cleared_counts: fn(&NetworkQueueClearStats) -> (usize, usize),
    driver_name: &str,
    rx_label: &str,
    tx_label: &str,
) -> bool {
    aethercore::modules::drivers::network::set_driver_io_owned(false);
    let cleared = aethercore::modules::drivers::clear_network_driver_queues(driver_kind);
    let _ = DriverLifecycle::teardown(runtime_driver);
    let ok = DriverLifecycle::init_driver(runtime_driver).is_ok();
    let (rx_count, tx_count) = cleared_counts(&cleared);

    if ok {
        register_dataplane();
        aethercore::modules::drivers::network::set_driver_io_owned(true);
    }

    aethercore::modules::drivers::note_rebind_result(driver_kind, ok);

    if ok {
        aethercore::klog_info!(
            "{} rebind success: cleared({}={},{}={})",
            driver_name,
            rx_label,
            rx_count,
            tx_label,
            tx_count
        );
    } else {
        aethercore::klog_warn!(
            "{} rebind failed after queue clear({}={},{}={})",
            driver_name,
            rx_label,
            rx_count,
            tx_label,
            tx_count
        );
    }

    ok
}
