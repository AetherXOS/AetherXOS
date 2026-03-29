use super::*;

#[test_case]
fn aarch64_policy_can_disable_advanced_virtualization_operations() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_virtualization_snapshot_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_dirty_logging_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_live_migration_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_nested_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_time_virtualization_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_device_passthrough_enabled(Some(false));
    crate::config::KernelConfig::set_virtualization_execution_policy_profile(Some(
        crate::config::VirtualizationExecutionProfile {
            scheduling_class: crate::config::VirtualizationExecutionClass::Background,
        },
    ));

    let mut virt = base_el2_virt();
    virt.vm_launch_ready = true;
    virt.prep_attempts = 1;
    virt.prep_success = 1;
    virt.vmx_lifecycle = "active";
    virt.svm_lifecycle = "active";
    let gic = crate::hal::aarch64::gic::GicStats {
        initialized: true,
        version: 3,
    };
    let timer = base_timer(1_000_000);
    let status = virt_platform_status(virt, gic, timer, true);
    assert!(!status.exit_tracing_ready);
    assert!(!status.snapshot_ready);
    assert!(!status.dirty_logging_ready);
    assert!(!status.live_migration_ready);
    assert_eq!(status.runtime_step, "resume-el2-guest-basic");
    assert_eq!(status.runtime_selected_mode, "backend-blocked");
    assert_eq!(status.runtime_strategy, "conservative-hold");
    assert_eq!(status.runtime_budget_class, "minimal");
    assert_eq!(status.runtime_dispatch_class, "conservative");
    assert_eq!(status.runtime_preemption_policy, "hold");
    assert_eq!(status.runtime_scheduler_lane, "background");
    assert_eq!(status.runtime_dispatch_window, "hold-window");
    assert_eq!(status.runtime_execution_profile, "Background");
    assert_eq!(status.runtime_execution_profile_scope, "runtime-limited");
    assert_eq!(status.runtime_governor_profile, "Balanced");
    assert_eq!(status.runtime_governor_profile_scope, "fully-enabled");
    assert_eq!(status.runtime_governor_class, "background-optimized");
    assert_eq!(status.runtime_latency_bias, "relaxed");
    assert_eq!(status.runtime_energy_bias, "saving");
    assert_eq!(status.runtime_aux_step, "no-aux-step");
    assert_eq!(status.policy_scope, "runtime-limited");
    assert_eq!(status.advanced_operations_tier, "disabled");
    assert!(!status.nested_ready);
    assert!(!status.time_virtualization_ready);
    assert!(!status.device_passthrough_ready);

    crate::config::KernelConfig::reset_runtime_overrides();
}
