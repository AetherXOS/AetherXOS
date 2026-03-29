pub(super) fn log_network_driver_initialized(name: &str) {
    hypercore::klog_info!("{} network card initialized", name);
}

pub(super) fn log_network_driver_failure_for(
    driver_name: &str,
    context: &str,
    status: &hypercore::modules::drivers::DriverStatus,
) {
    let prefix = alloc::format!("{} {}", driver_name, context);
    log_network_driver_failure(prefix.as_str(), status);
}

pub(super) fn log_network_driver_failure(
    prefix: &str,
    status: &hypercore::modules::drivers::DriverStatus,
) {
    hypercore::klog_warn!(
        "{} health={:?} state={:?} err={:?} faults={} rec_budget={}/{} cooldown={}",
        prefix,
        status.health,
        status.state,
        status.last_error,
        status.fault_count,
        status.recovery_budget_remaining,
        status.recovery_budget_total,
        status.recovery_cooldown_ticks_remaining
    );
}

pub(super) fn log_network_driver_absent(prefix: &str, kind: impl core::fmt::Debug) {
    hypercore::klog_info!("{} {:?}", prefix, kind);
}

pub(super) fn log_network_probe_discovery(
    driver: &hypercore::modules::drivers::ProbedNetworkDriver,
) {
    match driver {
        hypercore::modules::drivers::ProbedNetworkDriver::VirtIo(net) => {
            hypercore::klog_info!(
                "VirtIO found network card at IO {:04x}, init...",
                net.io_base
            );
        }
        hypercore::modules::drivers::ProbedNetworkDriver::E1000(e1000) => {
            hypercore::klog_info!(
                "E1000 found network card: dev={:#06x} mmio={:#x}, init...",
                e1000.device_id,
                e1000.mmio_base
            );
        }
    }
}

pub(super) fn log_virtio_driver_runtime(net: &hypercore::modules::drivers::VirtIoNet) {
    let ctrl_ready = net.control_queue_available();
    let (ctrl_ops, ctrl_failures) = net.control_queue_stats();
    let mac = net.mac_address();
    hypercore::klog_info!(
        "VirtIO runtime: ctrl_queue_ready={} ctrl_ops={} ctrl_failures={} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        ctrl_ready,
        ctrl_ops,
        ctrl_failures,
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );
}
