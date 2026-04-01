use crate::hal::common::virt::{
    VirtStatus, BACKEND_NONE, CAPABILITY_EL2_ACTIVE, CAPABILITY_EL2_DETECTED,
    CAPABILITY_EL2_ENABLED, FEATURE_EL2_ACTIVE, FEATURE_EL2_DETECTED, FEATURE_EL2_ENABLED,
};

pub(super) fn capability_detail(
    status: VirtStatus,
    gic_initialized: bool,
    timer_frequency_hz: u64,
) -> &'static str {
    if status.caps.hypervisor_present {
        if status.vm_launch_ready && gic_initialized && timer_frequency_hz != 0 {
            CAPABILITY_EL2_ACTIVE
        } else if status.vm_launch_ready {
            CAPABILITY_EL2_ENABLED
        } else {
            CAPABILITY_EL2_DETECTED
        }
    } else {
        BACKEND_NONE
    }
}

pub(super) fn feature_detail(
    status: VirtStatus,
    gic_initialized: bool,
    timer_frequency_hz: u64,
) -> &'static str {
    let policy = crate::config::KernelConfig::virtualization_effective_profile();
    if status.caps.hypervisor_present {
        if status.vm_launch_ready && gic_initialized && timer_frequency_hz != 0 {
            if policy.trap_tracing && policy.live_migration {
                FEATURE_EL2_ACTIVE
            } else {
                FEATURE_EL2_ENABLED
            }
        } else if status.vm_launch_ready {
            FEATURE_EL2_ENABLED
        } else {
            FEATURE_EL2_DETECTED
        }
    } else {
        BACKEND_NONE
    }
}

#[cfg(test)]
mod tests {
    use super::{capability_detail, feature_detail};
    use crate::hal::common::virt::{
        VirtCaps, VirtEnableState, VirtStatus, CAPABILITY_EL2_ACTIVE, CAPABILITY_EL2_DETECTED,
        CAPABILITY_EL2_ENABLED, FEATURE_EL2_ACTIVE, FEATURE_EL2_DETECTED, FEATURE_EL2_ENABLED,
    };

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
            prep_attempts: 0,
            prep_success: 0,
            prep_failures: 0,
            vmx_lifecycle: "active",
            svm_lifecycle: "active",
        }
    }

    #[test_case]
    fn el2_capability_detail_prefers_full_path() {
        let status = el2_status();
        assert_eq!(
            capability_detail(status, true, 1_000_000),
            CAPABILITY_EL2_ACTIVE
        );
        assert_eq!(feature_detail(status, true, 1_000_000), FEATURE_EL2_ACTIVE);
    }

    #[test_case]
    fn el2_capability_detail_has_launch_only_path() {
        let status = el2_status();
        assert_eq!(
            capability_detail(status, false, 1_000_000),
            CAPABILITY_EL2_ENABLED
        );
        assert_eq!(
            feature_detail(status, false, 1_000_000),
            FEATURE_EL2_ENABLED
        );
    }

    #[test_case]
    fn el2_capability_detail_has_detected_only_path() {
        let mut status = el2_status();
        status.vm_launch_ready = false;
        assert_eq!(capability_detail(status, false, 0), CAPABILITY_EL2_DETECTED);
        assert_eq!(feature_detail(status, false, 0), FEATURE_EL2_DETECTED);
    }

    #[test_case]
    fn policy_can_downgrade_el2_feature_detail_from_active_to_enabled() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_virtualization_live_migration_enabled(Some(false));

        let status = el2_status();
        assert_eq!(
            capability_detail(status, true, 1_000_000),
            CAPABILITY_EL2_ACTIVE
        );
        assert_eq!(feature_detail(status, true, 1_000_000), FEATURE_EL2_ENABLED);

        crate::config::KernelConfig::reset_runtime_overrides();
    }
}
