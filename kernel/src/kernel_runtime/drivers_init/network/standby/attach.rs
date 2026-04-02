use super::super::super::super::*;
use super::super::irq::register_network_irq_handler;
use super::super::logging::{log_network_driver_absent, log_network_driver_failure};
use super::policy::{resolve_standby_policy, standby_driver_ready};

pub(super) fn init_standby_network_driver(
    runtime: &KernelRuntime,
    devices: &[aethercore::hal::pci::PciDevice],
    telemetry_drivers: bool,
    policy: aethercore::modules::drivers::NetworkDriverPolicy,
) {
    let Some((fallback_kind, fallback_policy)) = resolve_standby_policy(policy) else {
        return;
    };

    if standby_driver_ready(fallback_kind) {
        return;
    }

    if let Some(mut fallback) =
        aethercore::modules::drivers::probe_network_driver_with_policy(devices, fallback_policy)
    {
        initialize_standby_driver(runtime, telemetry_drivers, fallback);
    } else if telemetry_drivers {
        log_network_driver_absent(
            "No standby network driver found for fallback kind",
            fallback_kind,
        );
    }
}

fn initialize_standby_driver(
    runtime: &KernelRuntime,
    telemetry_drivers: bool,
    mut fallback: aethercore::modules::drivers::ProbedNetworkDriver,
) {
    if fallback.init_driver().is_ok() {
        let irq_line = fallback.irq_line();
        let standby_kind = fallback.active_kind();
        let attached = aethercore::modules::drivers::hotplug_attach_network_driver(fallback);
        if telemetry_drivers {
            aethercore::klog_info!(
                "Standby network driver initialized: kind={:?} attached={:?} active={:?}",
                standby_kind,
                attached,
                aethercore::modules::drivers::active_network_driver()
            );
        }
        register_network_irq_handler(runtime, standby_kind, irq_line);
    } else if telemetry_drivers {
        let st = fallback.status();
        log_network_driver_failure("Standby network init failed", &st);
    }
}
