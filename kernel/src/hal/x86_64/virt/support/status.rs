use super::*;

pub(crate) fn status() -> VirtStatus {
    let caps_bits = VIRT_CAPS_BITS.load(Ordering::Relaxed);
    let enable_bits = VIRT_ENABLE_BITS.load(Ordering::Relaxed);
    let ready = VIRT_VM_LAUNCH_READY.load(Ordering::Relaxed);
    let blocker = VIRT_VM_BLOCKER.load(Ordering::Relaxed);
    let (prep_attempts, prep_success, prep_failures) = crate::hal::common::virt::prep_success_state(
        VIRT_PREP_ATTEMPTS.load(Ordering::Relaxed),
        VIRT_PREP_SUCCESS.load(Ordering::Relaxed),
        VIRT_PREP_FAILURES.load(Ordering::Relaxed),
    );

    VirtStatus {
        caps: bits_to_caps(caps_bits),
        enabled: bits_to_enable(enable_bits),
        vm_launch_ready: ready,
        blocker: blocker_reason(blocker),
        vmx_vmcs_ready: VMX_VMCS_READY.load(Ordering::Relaxed),
        svm_vmcb_ready: SVM_VMCB_READY.load(Ordering::Relaxed),
        prep_attempts,
        prep_success,
        prep_failures,
        vmx_lifecycle: lifecycle_reason(VMX_LIFECYCLE.load(Ordering::Relaxed)),
        svm_lifecycle: lifecycle_reason(SVM_LIFECYCLE.load(Ordering::Relaxed)),
    }
}
