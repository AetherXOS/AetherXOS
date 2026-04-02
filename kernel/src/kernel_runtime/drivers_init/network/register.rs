use super::irq::register_network_irq_handler;
use crate::kernel_runtime::KernelRuntime;

pub(super) fn register_active_network_driver(
    runtime: &KernelRuntime,
    driver: aethercore::modules::drivers::ProbedNetworkDriver,
    irq_line: u8,
    active_kind: aethercore::modules::drivers::ActiveNetworkDriver,
) {
    #[cfg(feature = "networking")]
    {
        let registered = aethercore::modules::drivers::hotplug_attach_network_driver(driver);
        match registered {
            aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
                aethercore::modules::drivers::register_virtio_network_dataplane();
            }
            aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
                aethercore::modules::drivers::register_e1000_network_dataplane();
            }
            aethercore::modules::drivers::ActiveNetworkDriver::None => {}
        }
        aethercore::modules::drivers::network::set_driver_io_owned(true);
    }
    #[cfg(not(feature = "networking"))]
    let _ = driver;

    register_network_irq_handler(runtime, active_kind, irq_line);
}
