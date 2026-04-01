use crate::hal::common::virt::{
    VirtStatus, CONTROL_DETECTED, CONTROL_NONE, CONTROL_SVM_ENABLED, CONTROL_VMX_ACTIVE,
    CONTROL_VMX_ENABLED,
};

pub(super) fn control_detail(status: VirtStatus) -> &'static str {
    if status.enabled.vmxon_active {
        CONTROL_VMX_ACTIVE
    } else if status.enabled.vmx_enabled {
        CONTROL_VMX_ENABLED
    } else if status.enabled.svm_enabled {
        CONTROL_SVM_ENABLED
    } else if status.caps.vmx || status.caps.svm {
        CONTROL_DETECTED
    } else {
        CONTROL_NONE
    }
}
