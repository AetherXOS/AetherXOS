pub(super) fn log_network_driver_initialized(name: &str) {
    aethercore::klog_info!("{} network card initialized", name);
}

pub(super) fn log_network_driver_failure_for(
    driver_name: &str,
    context: &str,
    status: &aethercore::modules::drivers::DriverStatus,
) {
    let prefix = alloc::format!("{} {}", driver_name, context);
    log_network_driver_failure(prefix.as_str(), status);
}

pub(super) fn log_network_driver_failure(
    prefix: &str,
    status: &aethercore::modules::drivers::DriverStatus,
) {
    aethercore::klog_warn!(
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
    aethercore::klog_info!("{} {:?}", prefix, kind);
}

pub(super) fn log_network_probe_discovery(
    driver: &aethercore::modules::drivers::ProbedNetworkDriver,
) {
    match driver {
        aethercore::modules::drivers::ProbedNetworkDriver::VirtIo(net) => {
            aethercore::klog_info!(
                "VirtIO found network card at IO {:04x}, init...",
                net.io_base
            );
        }
        aethercore::modules::drivers::ProbedNetworkDriver::E1000(e1000) => {
            aethercore::klog_info!(
                "E1000 found network card: dev={:#06x} mmio={:#x}, init...",
                e1000.device_id,
                e1000.mmio_base
            );
        }
    }
}

pub(super) fn log_virtio_driver_runtime(net: &aethercore::modules::drivers::VirtIoNet) {
    let ctrl_ready = net.control_queue_available();
    let (ctrl_ops, ctrl_failures) = net.control_queue_stats();
    let mac = net.mac_address();
    aethercore::klog_info!(
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
