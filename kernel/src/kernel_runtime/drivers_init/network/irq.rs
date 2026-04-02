use crate::kernel_runtime::KernelRuntime;
use crate::kernel_runtime::interrupts::{e1000_irq_handler, virtio_irq_handler};

pub(super) fn register_network_irq_handler(
    runtime: &KernelRuntime,
    driver_kind: aethercore::modules::drivers::ActiveNetworkDriver,
    irq_line: u8,
) {
    #[cfg(feature = "dispatcher")]
    {
        let irq_base = aethercore::config::KernelConfig::irq_vector_base();
        let vector = irq_line.saturating_add(irq_base);
        match driver_kind {
            aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
                runtime
                    .dispatcher
                    .register_handler(vector, virtio_irq_handler);
            }
            aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
                runtime
                    .dispatcher
                    .register_handler(vector, e1000_irq_handler);
            }
            aethercore::modules::drivers::ActiveNetworkDriver::None => {}
        }
    }
    #[cfg(not(feature = "dispatcher"))]
    {
        let _ = runtime;
        let _ = driver_kind;
        let _ = irq_line;
    }
}
