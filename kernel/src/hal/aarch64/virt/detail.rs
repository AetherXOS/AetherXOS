mod capability;
mod control;
mod el2_detail;
mod interrupt;
mod launch;
mod policy;
mod time;
mod trap;

use crate::hal::common::virt::VirtStatus;
use crate::hal::common::virt::{
    control_is_operational, interrupt_is_operational, time_is_operational, trap_is_operational,
    ISOLATION_TIER_STAGE2_GICV3, ISOLATION_TIER_STAGE2_READY, STAGE_GUEST_RUNNABLE,
    STAGE_LAUNCH_PREPARED,
};

#[derive(Debug, Clone, Copy)]
pub struct VirtBackendDetail {
    pub backend_detail: &'static str,
    pub capability_detail: &'static str,
    pub feature_detail: &'static str,
    pub interrupt_detail: &'static str,
    pub time_detail: &'static str,
    pub capability_level: &'static str,
    pub launch_stage: &'static str,
    pub isolation_tier: &'static str,
    pub control_detail: &'static str,
    pub trap_detail: &'static str,
    pub operation_class: &'static str,
    pub backend_mode: &'static str,
    pub operational_tier: &'static str,
    pub policy_scope: &'static str,
    pub entry_policy_scope: &'static str,
    pub resume_policy_scope: &'static str,
    pub trap_dispatch_policy_scope: &'static str,
    pub nested_policy_scope: &'static str,
    pub time_virtualization_policy_scope: &'static str,
    pub device_passthrough_policy_scope: &'static str,
    pub entry_mode: &'static str,
    pub resume_mode: &'static str,
    pub trap_mode: &'static str,
    pub nested_mode: &'static str,
    pub time_mode: &'static str,
    pub device_passthrough_mode: &'static str,
}

impl VirtBackendDetail {
    pub fn operational_readiness(self) -> &'static str {
        crate::hal::common::virt::operational_readiness_from_stage(
            self.launch_stage,
            self.capability_level,
        )
    }

    pub fn summary_tuple(self) -> (&'static str, &'static str, &'static str) {
        (
            self.backend_detail,
            self.capability_level,
            self.operational_readiness(),
        )
    }

    pub fn can_launch_guest(self) -> bool {
        crate::hal::common::virt::can_launch_from_readiness(self.operational_readiness())
    }

    pub fn can_resume_guest(self) -> bool {
        matches!(
            self.launch_stage,
            STAGE_GUEST_RUNNABLE | STAGE_LAUNCH_PREPARED
        )
    }

    pub fn can_trap_guest(self) -> bool {
        trap_is_operational(self.trap_detail)
    }

    pub fn can_isolate_guest_memory(self) -> bool {
        self.isolation_tier == ISOLATION_TIER_STAGE2_GICV3
            || self.isolation_tier == ISOLATION_TIER_STAGE2_READY
    }

    pub fn can_control_guest(self) -> bool {
        control_is_operational(self.control_detail)
    }

    pub fn can_virtualize_interrupts(self) -> bool {
        interrupt_is_operational(self.interrupt_detail)
    }

    pub fn can_virtualize_time(self) -> bool {
        time_is_operational(self.time_detail)
    }

    pub fn operational_hooks(self) -> (&'static str, bool, bool, bool) {
        (
            self.operational_readiness(),
            self.can_control_guest(),
            self.can_virtualize_interrupts(),
            self.can_virtualize_time(),
        )
    }
}

pub fn summarize(
    status: VirtStatus,
    gic_initialized: bool,
    gic_version: u32,
    timer_frequency_hz: u64,
    memory_isolation_ready: bool,
) -> VirtBackendDetail {
    let backend_detail = if status.caps.hypervisor_present {
        el2_detail::backend_detail(status, gic_initialized, timer_frequency_hz)
    } else {
        "none"
    };

    let trap_handling_ready =
        el2_detail::trap_handling_ready(status, gic_initialized, timer_frequency_hz);

    let capability_level =
        policy::capability_level(status, trap_handling_ready, memory_isolation_ready);
    let launch_stage = launch::launch_stage(status, trap_handling_ready);
    let control_detail = control::control_detail(status, gic_initialized, timer_frequency_hz);
    let trap_detail = trap::trap_detail(status, trap_handling_ready, gic_initialized);
    let interrupt_detail = interrupt::interrupt_detail(status, gic_initialized, gic_version);
    let time_detail = time::time_detail(status, timer_frequency_hz);
    let isolation_tier = policy::isolation_tier(status, memory_isolation_ready, gic_version);
    let readiness =
        crate::hal::common::virt::operational_readiness_from_stage(launch_stage, capability_level);
    let scope_profile = crate::config::KernelConfig::virtualization_policy_scope_profile();
    let policy_scope = scope_profile.overall;
    let operation_class = crate::hal::common::virt::detail_operation_class(readiness, policy_scope);
    let entry_mode = crate::hal::common::virt::feature_backend_mode(
        crate::hal::common::virt::can_launch_from_readiness(readiness),
        scope_profile.entry,
        None,
    );
    let resume_mode = crate::hal::common::virt::feature_backend_mode(
        matches!(launch_stage, STAGE_GUEST_RUNNABLE | STAGE_LAUNCH_PREPARED),
        scope_profile.resume,
        Some(scope_profile.nested),
    );
    let trap_mode = crate::hal::common::virt::feature_backend_mode(
        trap_is_operational(trap_detail),
        scope_profile.trap_dispatch,
        Some(scope_profile.time_virtualization),
    );
    let nested_mode = crate::hal::common::virt::feature_backend_mode(
        trap_handling_ready,
        scope_profile.nested,
        None,
    );
    let time_mode = crate::hal::common::virt::feature_backend_mode(
        time_is_operational(time_detail),
        scope_profile.time_virtualization,
        None,
    );
    let device_passthrough_mode = crate::hal::common::virt::feature_backend_mode(
        isolation_tier == ISOLATION_TIER_STAGE2_GICV3
            || isolation_tier == ISOLATION_TIER_STAGE2_READY,
        scope_profile.device_passthrough,
        None,
    );

    VirtBackendDetail {
        backend_detail,
        capability_detail: capability::capability_detail(
            status,
            gic_initialized,
            timer_frequency_hz,
        ),
        feature_detail: capability::feature_detail(status, gic_initialized, timer_frequency_hz),
        interrupt_detail,
        time_detail,
        capability_level,
        launch_stage,
        isolation_tier,
        control_detail,
        trap_detail,
        operation_class,
        backend_mode: crate::hal::common::virt::backend_mode_from_class(
            backend_detail,
            operation_class,
        ),
        operational_tier: crate::hal::common::virt::operational_tier_from_class(
            readiness,
            capability_level,
            operation_class,
            policy_scope,
        ),
        policy_scope,
        entry_policy_scope: scope_profile.entry,
        resume_policy_scope: scope_profile.resume,
        trap_dispatch_policy_scope: scope_profile.trap_dispatch,
        nested_policy_scope: scope_profile.nested,
        time_virtualization_policy_scope: scope_profile.time_virtualization,
        device_passthrough_policy_scope: scope_profile.device_passthrough,
        entry_mode,
        resume_mode,
        trap_mode,
        nested_mode,
        time_mode,
        device_passthrough_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::summarize;
    use crate::hal::common::virt::{VirtCaps, VirtEnableState, VirtStatus};

    fn el2_status() -> VirtStatus {
        VirtStatus {
            caps: VirtCaps {
                vmx: false,
                svm: false,
                hypervisor_present: true,
            },
            enabled: VirtEnableState::default(),
            vm_launch_ready: true,
            blocker: "none",
            vmx_vmcs_ready: false,
            svm_vmcb_ready: false,
            prep_attempts: 1,
            prep_success: 1,
            prep_failures: 0,
            vmx_lifecycle: "active",
            svm_lifecycle: "active",
        }
    }

    fn el2_detected_only() -> VirtStatus {
        VirtStatus {
            caps: VirtCaps {
                vmx: false,
                svm: false,
                hypervisor_present: true,
            },
            enabled: VirtEnableState::default(),
            vm_launch_ready: false,
            blocker: "EL2 Not Active",
            vmx_vmcs_ready: false,
            svm_vmcb_ready: false,
            prep_attempts: 1,
            prep_success: 0,
            prep_failures: 1,
            vmx_lifecycle: "failed",
            svm_lifecycle: "failed",
        }
    }

    fn el2_launch_prepared() -> VirtStatus {
        VirtStatus {
            caps: VirtCaps {
                vmx: false,
                svm: false,
                hypervisor_present: true,
            },
            enabled: VirtEnableState::default(),
            vm_launch_ready: true,
            blocker: "gic-not-ready",
            vmx_vmcs_ready: false,
            svm_vmcb_ready: false,
            prep_attempts: 1,
            prep_success: 1,
            prep_failures: 0,
            vmx_lifecycle: "active",
            svm_lifecycle: "active",
        }
    }

    #[test_case]
    fn summarize_el2_path_stays_consistent() {
        let detail = summarize(el2_status(), true, 3, 1_000_000, true);
        assert_eq!(detail.backend_detail, "el2:active+gic+timer");
        assert_eq!(detail.capability_detail, "el2:timer+gic+entry");
        assert_eq!(detail.feature_detail, "el2:vgic+vtimer");
        assert_eq!(detail.interrupt_detail, "gicv3-virt-ready");
        assert_eq!(detail.time_detail, "cntv-virtual-time-ready");
        assert_eq!(detail.control_detail, "el2-control-active");
        assert_eq!(detail.trap_detail, "el2-traps-ready");
        assert_eq!(detail.operation_class, "full");
        assert_eq!(detail.backend_mode, "backend-full");
        assert_eq!(detail.operational_tier, "production");
        assert_eq!(detail.entry_policy_scope, "fully-enabled");
        assert_eq!(detail.trap_dispatch_policy_scope, "fully-enabled");
        assert_eq!(detail.entry_mode, "backend-full");
        assert_eq!(detail.resume_mode, "backend-full");
        assert_eq!(detail.trap_mode, "backend-full");
        assert_eq!(detail.operational_readiness(), "ready");
        assert!(detail.can_control_guest());
        assert!(detail.can_virtualize_interrupts());
        assert!(detail.can_virtualize_time());
        assert_eq!(
            detail.summary_tuple(),
            ("el2:active+gic+timer", "tier3", "ready")
        );
        assert_eq!(detail.operational_hooks(), ("ready", true, true, true));
    }

    #[test_case]
    fn summarize_el2_detected_only_path_is_blocked() {
        let detail = summarize(el2_detected_only(), false, 0, 0, false);
        assert_eq!(detail.backend_detail, "el2:detected");
        assert_eq!(detail.capability_detail, "el2:present");
        assert_eq!(detail.feature_detail, "el2:basic-discovery");
        assert_eq!(detail.interrupt_detail, "interrupt-basic");
        assert_eq!(detail.time_detail, "time-basic");
        assert_eq!(detail.control_detail, "el2-detected");
        assert_eq!(detail.trap_detail, "trap-not-ready");
        assert_eq!(detail.operation_class, "blocked");
        assert_eq!(detail.backend_mode, "backend-blocked");
        assert_eq!(detail.operational_tier, "unavailable");
        assert_eq!(detail.operational_readiness(), "blocked");
        assert!(!detail.can_launch_guest());
        assert!(!detail.can_resume_guest());
        assert!(!detail.can_trap_guest());
        assert!(!detail.can_isolate_guest_memory());
        assert!(!detail.can_control_guest());
        assert!(!detail.can_virtualize_interrupts());
        assert!(!detail.can_virtualize_time());
        assert_eq!(detail.summary_tuple(), ("el2:detected", "tier1", "blocked"));
        assert_eq!(detail.operational_hooks(), ("blocked", false, false, false));
    }

    #[test_case]
    fn summarize_el2_launch_prepared_path_is_staged() {
        let detail = summarize(el2_launch_prepared(), false, 0, 1_000_000, false);
        assert_eq!(detail.backend_detail, "el2:active");
        assert_eq!(detail.capability_level, "tier1");
        assert_eq!(detail.launch_stage, "launch-prepared");
        assert_eq!(detail.operation_class, "basic");
        assert_eq!(detail.backend_mode, "backend-basic");
        assert_eq!(detail.operational_tier, "degraded");
        assert_eq!(detail.resume_policy_scope, "fully-enabled");
        assert_eq!(detail.entry_mode, "backend-full");
        assert_eq!(detail.trap_mode, "backend-blocked");
        assert_eq!(detail.operational_readiness(), "staged");
        assert!(detail.can_launch_guest());
        assert!(detail.can_resume_guest());
        assert!(!detail.can_trap_guest());
        assert!(!detail.can_isolate_guest_memory());
        assert!(detail.can_control_guest());
        assert!(!detail.can_virtualize_interrupts());
        assert!(!detail.can_virtualize_time());
        assert_eq!(detail.summary_tuple(), ("el2:active", "tier1", "staged"));
        assert_eq!(detail.operational_hooks(), ("staged", true, false, false));
    }

    #[test_case]
    fn el2_transition_progression_stays_ordered() {
        let progression = [
            summarize(el2_detected_only(), false, 0, 0, false).operational_readiness(),
            summarize(el2_detected_only(), false, 0, 1_000_000, false).operational_readiness(),
            summarize(el2_launch_prepared(), false, 0, 1_000_000, false).operational_readiness(),
            summarize(el2_status(), true, 3, 1_000_000, true).operational_readiness(),
        ];
        assert_eq!(progression, ["partial", "partial", "staged", "ready"]);
    }

    #[test_case]
    fn detail_reflects_runtime_policy_scopes() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_virtualization_nested_enabled(Some(false));
        crate::config::KernelConfig::set_virtualization_time_virtualization_enabled(Some(false));
        crate::config::KernelConfig::set_virtualization_device_passthrough_enabled(Some(false));

        let detail = summarize(el2_status(), true, 3, 1_000_000, true);
        assert_eq!(detail.policy_scope, "runtime-limited");
        assert_eq!(detail.nested_policy_scope, "runtime-limited");
        assert_eq!(detail.time_virtualization_policy_scope, "runtime-limited");
        assert_eq!(detail.device_passthrough_policy_scope, "runtime-limited");
        assert_eq!(detail.resume_mode, "backend-basic");
        assert_eq!(detail.trap_mode, "backend-basic");
        assert_eq!(detail.device_passthrough_mode, "backend-basic");

        crate::config::KernelConfig::reset_runtime_overrides();
    }
}
