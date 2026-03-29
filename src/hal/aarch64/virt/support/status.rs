use super::*;

pub(crate) fn status() -> VirtStatus {
    let caps = VirtCaps {
        vmx: false,
        svm: false,
        hypervisor_present: HYP_ACTIVE.load(Ordering::Relaxed)
            || HYP_SUPPORTED.load(Ordering::Relaxed),
    };
    let ready = VM_LAUNCH_READY.load(Ordering::Relaxed);
    let (prep_attempts, prep_success, prep_failures) = crate::hal::common::virt::prep_success_state(
        PREP_ATTEMPTS.load(Ordering::Relaxed),
        PREP_SUCCESS.load(Ordering::Relaxed),
        PREP_FAILURES.load(Ordering::Relaxed),
    );

    VirtStatus {
        caps,
        enabled: VirtEnableState::default(),
        vm_launch_ready: ready,
        blocker: blocker_reason(VM_BLOCKER.load(Ordering::Relaxed)),
        vmx_vmcs_ready: false,
        svm_vmcb_ready: false,
        prep_attempts,
        prep_success,
        prep_failures,
        vmx_lifecycle: lifecycle_reason(EL2_LIFECYCLE.load(Ordering::Relaxed)),
        svm_lifecycle: lifecycle_reason(EL2_LIFECYCLE.load(Ordering::Relaxed)),
    }
}
