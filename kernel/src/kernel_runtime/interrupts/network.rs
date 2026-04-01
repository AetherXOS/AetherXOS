#[cfg(feature = "drivers")]
pub(super) fn virtio_irq_handler(irq: u8) {
    handle_network_irq(
        irq,
        "VirtIO",
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo,
    );
}

#[cfg(feature = "drivers")]
pub(super) fn e1000_irq_handler(irq: u8) {
    handle_network_irq(
        irq,
        "E1000",
        hypercore::modules::drivers::ActiveNetworkDriver::E1000,
    );
}

#[cfg(feature = "drivers")]
fn handle_network_irq(
    irq: u8,
    label: &'static str,
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
) {
    #[cfg(feature = "networking")]
    use crate::kernel_runtime::network_remediation::service_specific_network_driver_io;

    if hypercore::generated_consts::CORE_ENABLE_IRQ_TRACE {
        hypercore::klog_trace!("{} IRQ vector {}", label, irq);
    }
    #[cfg(feature = "networking")]
    {
        if !service_specific_network_driver_io(driver) {
            hypercore::modules::drivers::service_network_irq(driver);
        }
    }
    #[cfg(not(feature = "networking"))]
    {
        hypercore::modules::drivers::service_network_irq(driver);
    }
}
