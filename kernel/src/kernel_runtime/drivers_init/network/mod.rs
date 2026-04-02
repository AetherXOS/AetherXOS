use crate::kernel_runtime::KernelRuntime;

mod irq;
mod logging;
mod probe;
mod register;
#[cfg(feature = "networking")]
mod setup;
#[cfg(feature = "networking")]
mod standby;
#[cfg(feature = "networking")]
mod telemetry;

pub(super) fn init_network_drivers(
    runtime: &KernelRuntime,
    devices: &[aethercore::hal::pci::PciDevice],
    telemetry_drivers: bool,
) {
    #[cfg(feature = "networking")]
    {
        setup::configure_network_runtime_defaults();
    }

    let policy = probe::probe_and_init_primary_driver(runtime, devices, telemetry_drivers);

    #[cfg(feature = "networking")]
    standby::init_standby_network_driver(runtime, devices, telemetry_drivers, policy);

    #[cfg(feature = "networking")]
    if telemetry_drivers {
        telemetry::log_network_runtime_dashboard();
    }
}
