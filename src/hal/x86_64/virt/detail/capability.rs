use crate::hal::common::virt::{
    VirtStatus, BACKEND_NONE, CAPABILITY_SVM_ACTIVE, CAPABILITY_SVM_DETECTED,
    CAPABILITY_SVM_ENABLED, CAPABILITY_VMX_ACTIVE, CAPABILITY_VMX_DETECTED, CAPABILITY_VMX_ENABLED,
    FEATURE_SVM_ACTIVE, FEATURE_SVM_DETECTED, FEATURE_SVM_ENABLED, FEATURE_VMX_ACTIVE,
    FEATURE_VMX_DETECTED, FEATURE_VMX_ENABLED,
};

pub(super) fn capability_detail(status: VirtStatus) -> &'static str {
    if status.caps.vmx {
        if status.enabled.vmxon_active && status.vmx_vmcs_ready {
            CAPABILITY_VMX_ACTIVE
        } else if status.enabled.vmx_enabled {
            CAPABILITY_VMX_ENABLED
        } else {
            CAPABILITY_VMX_DETECTED
        }
    } else if status.caps.svm {
        if status.enabled.svm_enabled && status.svm_vmcb_ready {
            CAPABILITY_SVM_ACTIVE
        } else if status.enabled.svm_enabled {
            CAPABILITY_SVM_ENABLED
        } else {
            CAPABILITY_SVM_DETECTED
        }
    } else {
        BACKEND_NONE
    }
}

pub(super) fn feature_detail(status: VirtStatus) -> &'static str {
    let policy = crate::config::KernelConfig::virtualization_effective_profile();
    if status.caps.vmx {
        if status.enabled.vmxon_active && status.vmx_vmcs_ready {
            if policy.trap_tracing && policy.dirty_logging {
                FEATURE_VMX_ACTIVE
            } else {
                FEATURE_VMX_ENABLED
            }
        } else if status.enabled.vmx_enabled {
            FEATURE_VMX_ENABLED
        } else {
            FEATURE_VMX_DETECTED
        }
    } else if status.caps.svm {
        if status.enabled.svm_enabled && status.svm_vmcb_ready {
            if policy.trap_tracing && policy.dirty_logging {
                FEATURE_SVM_ACTIVE
            } else {
                FEATURE_SVM_ENABLED
            }
        } else if status.enabled.svm_enabled {
            FEATURE_SVM_ENABLED
        } else {
            FEATURE_SVM_DETECTED
        }
    } else {
        BACKEND_NONE
    }
}

#[cfg(test)]
mod tests {
    use super::{capability_detail, feature_detail};
    use crate::hal::common::virt::{
        VirtCaps, VirtEnableState, VirtStatus, CAPABILITY_SVM_ACTIVE, CAPABILITY_SVM_ENABLED,
        CAPABILITY_VMX_ACTIVE, CAPABILITY_VMX_ENABLED, FEATURE_SVM_ACTIVE, FEATURE_SVM_ENABLED,
        FEATURE_VMX_ACTIVE, FEATURE_VMX_ENABLED,
    };

    fn base_status() -> VirtStatus {
        VirtStatus {
            caps: VirtCaps::default(),
            enabled: VirtEnableState::default(),
            vm_launch_ready: false,
            blocker: "none",
            vmx_vmcs_ready: false,
            svm_vmcb_ready: false,
            prep_attempts: 0,
            prep_success: 0,
            prep_failures: 0,
            vmx_lifecycle: "uninitialized",
            svm_lifecycle: "uninitialized",
        }
    }

    #[test_case]
    fn vmx_capability_detail_prefers_active_path() {
        let mut status = base_status();
        status.caps.vmx = true;
        status.enabled.vmx_enabled = true;
        status.enabled.vmxon_active = true;
        status.vmx_vmcs_ready = true;
        assert_eq!(capability_detail(status), CAPABILITY_VMX_ACTIVE);
        assert_eq!(feature_detail(status), FEATURE_VMX_ACTIVE);
    }

    #[test_case]
    fn svm_capability_detail_prefers_vmcb_path() {
        let mut status = base_status();
        status.caps.svm = true;
        status.enabled.svm_enabled = true;
        status.svm_vmcb_ready = true;
        assert_eq!(capability_detail(status), CAPABILITY_SVM_ACTIVE);
        assert_eq!(feature_detail(status), FEATURE_SVM_ACTIVE);
    }

    #[test_case]
    fn vmx_capability_detail_has_intermediate_enabled_path() {
        let mut status = base_status();
        status.caps.vmx = true;
        status.enabled.vmx_enabled = true;
        assert_eq!(capability_detail(status), CAPABILITY_VMX_ENABLED);
        assert_eq!(feature_detail(status), FEATURE_VMX_ENABLED);
    }

    #[test_case]
    fn svm_capability_detail_has_intermediate_enabled_path() {
        let mut status = base_status();
        status.caps.svm = true;
        status.enabled.svm_enabled = true;
        assert_eq!(capability_detail(status), CAPABILITY_SVM_ENABLED);
        assert_eq!(feature_detail(status), FEATURE_SVM_ENABLED);
    }

    #[test_case]
    fn policy_can_downgrade_x86_feature_detail_from_active_to_enabled() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_virtualization_trap_tracing_enabled(Some(false));

        let mut status = base_status();
        status.caps.vmx = true;
        status.enabled.vmx_enabled = true;
        status.enabled.vmxon_active = true;
        status.vmx_vmcs_ready = true;
        assert_eq!(capability_detail(status), CAPABILITY_VMX_ACTIVE);
        assert_eq!(feature_detail(status), FEATURE_VMX_ENABLED);

        crate::config::KernelConfig::reset_runtime_overrides();
    }
}
