use crate::kernel_runtime::KernelRuntime;
#[cfg(feature = "drivers")]
use aethercore::generated_consts::CORE_ENABLE_DRIVER_INIT;

#[cfg(feature = "drivers")]
mod network;
#[cfg(feature = "drivers")]
mod storage;

impl KernelRuntime {
    pub(super) fn init_drivers(&self, devices: &[aethercore::hal::pci::PciDevice]) {
        #[cfg(feature = "drivers")]
        {
            let telemetry_drivers = aethercore::config::KernelConfig::telemetry_drivers_enabled();

            storage::init_storage_drivers(devices, telemetry_drivers);

            if CORE_ENABLE_DRIVER_INIT {
                network::init_network_drivers(self, devices, telemetry_drivers);
            } else {
                aethercore::klog_info!("Driver initialization disabled by config");
            }
        }
        #[cfg(not(feature = "drivers"))]
        {
            let _ = self;
            let _ = devices;
            aethercore::klog_info!("Driver subsystem disabled by features");
        }
    }
}
