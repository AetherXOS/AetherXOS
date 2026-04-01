use crate::hal::common::virt::{VirtStatus, TIME_BASIC, TIME_NONE, TIME_SVM_READY, TIME_VMX_READY};

pub(super) fn time_detail(status: VirtStatus) -> &'static str {
    if status.enabled.vmxon_active && status.vmx_vmcs_ready {
        TIME_VMX_READY
    } else if status.enabled.svm_enabled && status.svm_vmcb_ready {
        TIME_SVM_READY
    } else if status.enabled.vmx_enabled || status.enabled.svm_enabled {
        TIME_BASIC
    } else {
        TIME_NONE
    }
}
