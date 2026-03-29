use crate::hal::common::virt::{
    VirtStatus, TRAP_NOT_READY, TRAP_STRUCTURES_READY, TRAP_SVM_READY, TRAP_VMX_READY,
};

pub(super) fn trap_detail(status: VirtStatus, trap_handling_ready: bool) -> &'static str {
    if trap_handling_ready && status.caps.vmx {
        TRAP_VMX_READY
    } else if trap_handling_ready && status.caps.svm {
        TRAP_SVM_READY
    } else if status.vmx_vmcs_ready || status.svm_vmcb_ready {
        TRAP_STRUCTURES_READY
    } else {
        TRAP_NOT_READY
    }
}
