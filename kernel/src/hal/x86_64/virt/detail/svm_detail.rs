use crate::hal::common::virt::{
    VirtStatus, BACKEND_SVM_ACTIVE, BACKEND_SVM_DETECTED, BACKEND_SVM_ENABLED,
};

pub(super) fn backend_detail(status: VirtStatus) -> &'static str {
    if status.enabled.svm_enabled && status.svm_vmcb_ready {
        BACKEND_SVM_ACTIVE
    } else if status.enabled.svm_enabled {
        BACKEND_SVM_ENABLED
    } else {
        BACKEND_SVM_DETECTED
    }
}

pub(super) fn trap_handling_ready(status: VirtStatus) -> bool {
    status.enabled.svm_enabled && status.svm_vmcb_ready
}
