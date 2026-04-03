mod server;
mod support;
mod virt;
pub mod irq;
use support::classify_platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Virt,
    Server,
    BareMetalUnknown,
}

#[derive(Debug, Clone, Copy)]
pub struct PlatformStatus {
    pub kind: PlatformKind,
    pub acpi_present: bool,
    pub dtb_present: bool,
    pub hypervisor_present: bool,
    pub gic_initialized: bool,
    pub cpu_count: usize,
    pub aps_ready: u32,
    pub timer_frequency_hz: u64,
    pub vm_launch_ready: bool,
    pub virt_backend: &'static str,
    pub virt_backend_detail: &'static str,
    pub virt_capability_detail: &'static str,
    pub virt_feature_detail: &'static str,
    pub virt_interrupt_detail: &'static str,
    pub virt_time_detail: &'static str,
    pub virt_runtime_step: &'static str,
    pub virt_runtime_selected_mode: &'static str,
    pub virt_runtime_aux_step: &'static str,
    pub virt_runtime_operation_class: &'static str,
    pub virt_runtime_strategy: &'static str,
    pub virt_runtime_budget_class: &'static str,
    pub virt_runtime_dispatch_class: &'static str,
    pub virt_runtime_preemption_policy: &'static str,
    pub virt_runtime_scheduler_lane: &'static str,
    pub virt_runtime_dispatch_window: &'static str,
    pub virt_runtime_execution_profile: &'static str,
    pub virt_runtime_execution_profile_scope: &'static str,
    pub virt_runtime_governor_profile: &'static str,
    pub virt_runtime_governor_profile_scope: &'static str,
    pub virt_runtime_governor_class: &'static str,
    pub virt_runtime_latency_bias: &'static str,
    pub virt_runtime_energy_bias: &'static str,
    pub virt_runtime_blocked_by: Option<&'static str>,
    pub virt_runtime_policy_limited_by: Option<&'static str>,
    pub virt_policy_scope: &'static str,
    pub virt_entry_policy_scope: &'static str,
    pub virt_resume_policy_scope: &'static str,
    pub virt_trap_dispatch_policy_scope: &'static str,
    pub virt_nested_policy_scope: &'static str,
    pub virt_time_virtualization_policy_scope: &'static str,
    pub virt_device_passthrough_policy_scope: &'static str,
    pub virt_entry_mode: &'static str,
    pub virt_resume_mode: &'static str,
    pub virt_trap_mode: &'static str,
    pub virt_nested_mode: &'static str,
    pub virt_time_mode: &'static str,
    pub virt_device_passthrough_mode: &'static str,
    pub virt_backend_mode: &'static str,
    pub virt_operational_tier: &'static str,
    pub virt_backend_capability_level: &'static str,
    pub virt_control_detail: &'static str,
    pub virt_trap_detail: &'static str,
    pub virt_detect_state: &'static str,
    pub virt_prepare_state: &'static str,
    pub virt_capability_state: &'static str,
    pub virt_feature_state: &'static str,
    pub virt_launch_state: &'static str,
    pub virt_resume_state: &'static str,
    pub virt_trap_state: &'static str,
    pub virt_lifecycle_summary: &'static str,
    pub virt_lifecycle_progress_per_mille: u16,
    pub virt_execution_backend: &'static str,
    pub virt_irq_backend: &'static str,
    pub virt_memory_backend: &'static str,
    pub virt_launch_path: &'static str,
    pub virt_launch_stage: &'static str,
    pub virt_hardware_accel: bool,
    pub virt_prep_success_rate_per_mille: u64,
    pub virt_blocker: &'static str,
    pub virt_nested_ready: bool,
    pub virt_control_plane_ready: bool,
    pub virt_exit_tracing_ready: bool,
    pub virt_interrupt_virtualization_ready: bool,
    pub virt_time_virtualization_ready: bool,
    pub virt_monitoring_ready: bool,
    pub virt_resume_ready: bool,
    pub virt_guest_entry_ready: bool,
    pub virt_state_save_restore_ready: bool,
    pub virt_trap_handling_ready: bool,
    pub virt_observability_tier: &'static str,
    pub virt_snapshot_ready: bool,
    pub virt_dirty_logging_ready: bool,
    pub virt_live_migration_ready: bool,
    pub virt_advanced_operations_tier: &'static str,
    pub virt_isolation_tier: &'static str,
    pub virt_memory_isolation_ready: bool,
    pub virt_device_passthrough_ready: bool,
    pub virt_operational_readiness: &'static str,
}

pub fn status() -> PlatformStatus {
    let virt = super::virt::status();
    let gic = super::gic::stats();
    let timer = super::timer::GenericTimer::stats();
    let smp = super::smp::boot_stats();
    let acpi_present = super::acpi_rsdp_addr().is_some();
    let dtb_present = super::dtb_addr().is_some();
    let kind = classify_platform(acpi_present, dtb_present, virt.caps.hypervisor_present);
    let virt_status = support::virt_platform_status(virt, gic, timer, dtb_present);
    match kind {
        PlatformKind::Virt => virt::status(acpi_present, dtb_present, virt, gic, timer, smp),
        PlatformKind::Server => server::status(acpi_present, dtb_present, virt, gic, timer, smp),
        PlatformKind::BareMetalUnknown => support::compose_platform_status(
            support::PlatformBaseStatus {
                kind,
                acpi_present,
                dtb_present,
                hypervisor_present: virt.caps.hypervisor_present,
                gic_initialized: gic.initialized,
                cpu_count: super::smp::cpu_count(),
                aps_ready: smp.aps_ready,
                timer_frequency_hz: timer.frequency_hz,
                vm_launch_ready: virt.vm_launch_ready,
            },
            virt_status,
        ),
    }
}
