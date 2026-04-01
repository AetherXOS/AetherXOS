use super::irq::register_network_irq_handler;
use crate::kernel_runtime::KernelRuntime;

pub(super) fn register_active_network_driver(
    runtime: &KernelRuntime,
    driver: hypercore::modules::drivers::ProbedNetworkDriver,
    irq_line: u8,
    active_kind: hypercore::modules::drivers::ActiveNetworkDriver,
) {
    #[cfg(feature = "networking")]
    {
        let registered = hypercore::modules::drivers::hotplug_attach_network_driver(driver);
        match registered {
            hypercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
                hypercore::modules::drivers::register_virtio_network_dataplane();
            }
            hypercore::modules::drivers::ActiveNetworkDriver::E1000 => {
                hypercore::modules::drivers::register_e1000_network_dataplane();
            }
            hypercore::modules::drivers::ActiveNetworkDriver::None => {}
        }
        hypercore::modules::drivers::network::set_driver_io_owned(true);
    }
    #[cfg(not(feature = "networking"))]
    let _ = driver;

    register_network_irq_handler(runtime, active_kind, irq_line);
}
