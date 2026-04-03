#[cfg(feature = "networking")]
use super::init_network_bridge_runtime;
#[cfg(target_arch = "aarch64")]
use super::log_aarch64_exception_runtime;
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
use super::log_x86_irq_runtime;
#[cfg(feature = "vfs")]
use super::log_vfs_runtime_sections;
use super::{
    PlatformTelemetryConfig, log_boundary_policy, log_core_runtime_limits, log_hal_wait_policy,
    log_library_surfaces, log_runtime_policy_summary, log_runtime_sections,
    log_virtualization_platform_lifecycle, log_virtualization_runtime_profile, log_watchdog_policy,
    should_log_library_inventory,
};

pub(crate) fn run_platform_runtime_orchestration(telemetry: PlatformTelemetryConfig) {
    let telemetry_runtime = telemetry.runtime;

    log_boundary_policy();
    log_core_runtime_limits();
    log_watchdog_policy();
    log_library_surfaces(should_log_library_inventory());
    aethercore::kernel::policy::apply_runtime_policy_preset();

    if telemetry_runtime {
        log_runtime_policy_summary();
        log_hal_wait_policy();
    }
    if telemetry.virtualization_runtime() {
        log_virtualization_runtime_profile();
    }
    if telemetry.platform_lifecycle_runtime() {
        log_virtualization_platform_lifecycle();
    }
    #[cfg(target_arch = "aarch64")]
    if telemetry_runtime {
        log_aarch64_exception_runtime();
    }
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    if telemetry_runtime {
        log_x86_irq_runtime();
    }

    #[cfg(feature = "networking")]
    init_network_bridge_runtime(telemetry);

    log_runtime_sections(telemetry, telemetry_runtime);

    #[cfg(feature = "vfs")]
    log_vfs_runtime_sections(telemetry, should_log_library_inventory());
}
