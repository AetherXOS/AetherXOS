use crate::hal::common::virt::{
    VirtStatus, INTERRUPT_BASIC, INTERRUPT_NONE, INTERRUPT_SVM_READY, INTERRUPT_VMX_READY,
};

pub(super) fn interrupt_detail(status: VirtStatus) -> &'static str {
    if status.caps.vmx && status.vmx_vmcs_ready {
        INTERRUPT_VMX_READY
    } else if status.caps.svm && status.svm_vmcb_ready {
        INTERRUPT_SVM_READY
    } else if status.caps.vmx || status.caps.svm {
        INTERRUPT_BASIC
    } else {
        INTERRUPT_NONE
    }
}

#[cfg(test)]
mod tests {
    use super::interrupt_detail;
    use crate::hal::common::virt::{
        VirtCaps, VirtEnableState, VirtStatus, INTERRUPT_SVM_READY, INTERRUPT_VMX_READY,
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
    fn interrupt_detail_prefers_vmx_then_svm() {
        let mut status = base_status();
        status.caps.vmx = true;
        status.vmx_vmcs_ready = true;
        assert_eq!(interrupt_detail(status), INTERRUPT_VMX_READY);
        status.caps.vmx = false;
        status.caps.svm = true;
        status.svm_vmcb_ready = true;
        assert_eq!(interrupt_detail(status), INTERRUPT_SVM_READY);
    }
}
