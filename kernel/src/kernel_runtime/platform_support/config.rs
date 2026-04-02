#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlatformTelemetryConfig {
    pub(crate) runtime: bool,
    pub(crate) virtualization: bool,
    pub(crate) platform_lifecycle: bool,
    pub(crate) scheduler: bool,
    pub(crate) power: bool,
    pub(crate) drivers: bool,
    #[cfg(feature = "networking")]
    pub(crate) network: bool,
    #[cfg(feature = "vfs")]
    pub(crate) vfs: bool,
}

impl PlatformTelemetryConfig {
    pub(crate) fn collect() -> Self {
        Self {
            runtime: aethercore::config::KernelConfig::telemetry_runtime_summary_enabled(),
            virtualization: aethercore::config::KernelConfig::telemetry_virtualization_enabled(),
            platform_lifecycle:
                aethercore::config::KernelConfig::telemetry_platform_lifecycle_enabled(),
            scheduler: aethercore::config::KernelConfig::telemetry_scheduler_enabled(),
            power: aethercore::config::KernelConfig::telemetry_power_enabled(),
            drivers: aethercore::config::KernelConfig::telemetry_drivers_enabled(),
            #[cfg(feature = "networking")]
            network: aethercore::config::KernelConfig::telemetry_network_enabled(),
            #[cfg(feature = "vfs")]
            vfs: aethercore::config::KernelConfig::telemetry_vfs_enabled(),
        }
    }

    pub(crate) fn scheduler_runtime(self) -> bool {
        self.runtime && self.scheduler
    }

    pub(crate) fn virtualization_runtime(self) -> bool {
        self.runtime && self.virtualization
    }

    pub(crate) fn platform_lifecycle_runtime(self) -> bool {
        self.virtualization_runtime() && self.platform_lifecycle
    }

    pub(crate) fn power_runtime(self) -> bool {
        self.runtime && self.power
    }

    pub(crate) fn driver_runtime(self) -> bool {
        self.runtime && self.drivers
    }

    #[cfg(feature = "networking")]
    pub(crate) fn network_runtime(self) -> bool {
        self.runtime && self.network
    }

    #[cfg(feature = "vfs")]
    pub(crate) fn vfs_runtime(self) -> bool {
        self.runtime && self.vfs
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PciAttachTelemetryConfig {
    pub(crate) security: bool,
    pub(crate) ipc: bool,
    #[cfg(feature = "networking")]
    pub(crate) network: bool,
}

impl PciAttachTelemetryConfig {
    pub(crate) fn collect() -> Self {
        Self {
            security: aethercore::config::KernelConfig::telemetry_security_enabled(),
            ipc: aethercore::config::KernelConfig::telemetry_ipc_enabled(),
            #[cfg(feature = "networking")]
            network: aethercore::config::KernelConfig::telemetry_network_enabled(),
        }
    }
}

#[inline(always)]
pub(crate) fn should_log_library_inventory() -> bool {
    aethercore::config::KernelConfig::should_log_library_inventory()
}

#[cfg(feature = "networking")]
#[inline(always)]
pub(crate) fn should_log_network_transport(telemetry: PciAttachTelemetryConfig) -> bool {
    telemetry.network
}

#[inline(always)]
pub(crate) fn should_log_security_telemetry(telemetry: PciAttachTelemetryConfig) -> bool {
    telemetry.security
}

#[inline(always)]
pub(crate) fn should_log_ipc_telemetry(telemetry: PciAttachTelemetryConfig) -> bool {
    telemetry.ipc
}
