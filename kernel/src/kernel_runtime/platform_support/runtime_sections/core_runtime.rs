use super::super::config::PlatformTelemetryConfig;

pub(crate) fn log_runtime_sections(telemetry: PlatformTelemetryConfig, telemetry_runtime: bool) {
    if telemetry.scheduler_runtime() {
        super::super::log_rt_preemption_guard();
    }

    if telemetry.driver_runtime() {
        super::super::log_module_loader_runtime();
    }

    #[cfg(feature = "allocators")]
    if telemetry.driver_runtime() {
        super::super::log_slab_runtime();
    }

    super::super::log_launch_pipeline();
    super::super::log_scheduler_runtime();

    if telemetry.power_runtime() {
        super::super::log_power_baseline();
        super::super::log_serial_runtime();
        super::super::log_watchdog_runtime();
    }

    #[cfg(feature = "allocators")]
    if telemetry.driver_runtime() {
        super::super::log_allocator_diagnostics();
    }

    #[cfg(feature = "ring_protection")]
    if telemetry_runtime {
        super::super::log_syscall_runtime();
    }

    #[cfg(feature = "dispatcher")]
    if telemetry.scheduler_runtime() {
        super::super::log_dispatcher_vectored_runtime();
    }

    #[cfg(feature = "dispatcher")]
    if telemetry_runtime && aethercore::config::KernelConfig::telemetry_ipc_enabled() {
        super::super::log_dispatcher_upcall_runtime();
    }

    if telemetry.scheduler_runtime() {
        super::super::log_load_balance_runtime();
    }
}
