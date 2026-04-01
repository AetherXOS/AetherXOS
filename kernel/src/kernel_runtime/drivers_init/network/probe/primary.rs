use super::super::register::register_active_network_driver;
use super::logging::{
    log_driver_init_failure, log_driver_init_success, log_network_driver_policy,
    log_network_probe_plan, log_probe_discovery, log_virtio_runtime,
};
use crate::kernel_runtime::KernelRuntime;

pub(super) fn probe_and_init_primary_driver(
    runtime: &KernelRuntime,
    devices: &[hypercore::hal::pci::PciDevice],
    telemetry_drivers: bool,
) -> hypercore::modules::drivers::NetworkDriverPolicy {
    use hypercore::modules::drivers::probe_network_driver_with_policy;

    if telemetry_drivers {
        log_network_probe_plan();
        log_network_driver_policy();
    }

    let policy = hypercore::modules::drivers::network_driver_policy();
    if let Some(mut driver) = probe_network_driver_with_policy(devices, policy) {
        log_probe_discovery(&driver);
        if driver.init_driver().is_ok() {
            let irq_line = driver.irq_line();
            let active_kind = driver.active_kind();
            log_driver_init_success(&driver);
            if telemetry_drivers {
                log_virtio_runtime(&driver);
            }
            register_active_network_driver(runtime, driver, irq_line, active_kind);
        } else {
            log_driver_init_failure(&driver);
        }
    } else if telemetry_drivers {
        hypercore::klog_warn!("No supported network driver found by probe catalog");
    }

    policy
}
