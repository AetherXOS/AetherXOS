use super::*;

pub(crate) fn initialize_launch_context() -> bool {
    let current = status();
    if let Some(code) = crate::hal::common::virt::activate_lifecycle_code(
        current.vm_launch_ready,
        current.caps.vmx && current.vmx_vmcs_ready,
    ) {
        VMX_LIFECYCLE.store(code, Ordering::Relaxed);
        return true;
    }

    if let Some(code) = crate::hal::common::virt::activate_lifecycle_code(
        current.vm_launch_ready,
        current.caps.svm && current.svm_vmcb_ready,
    ) {
        SVM_LIFECYCLE.store(code, Ordering::Relaxed);
        return true;
    }

    false
}

pub(crate) fn reset_launch_context() -> bool {
    let mut changed = false;

    if let Some(code) =
        crate::hal::common::virt::reset_lifecycle_code(VMX_VMCS_READY.load(Ordering::Relaxed))
    {
        VMX_LIFECYCLE.store(code, Ordering::Relaxed);
        changed = true;
    }

    if let Some(code) =
        crate::hal::common::virt::reset_lifecycle_code(SVM_VMCB_READY.load(Ordering::Relaxed))
    {
        SVM_LIFECYCLE.store(code, Ordering::Relaxed);
        changed = true;
    }

    changed
}

pub(crate) fn teardown_launch_context() {
    if let Some(code) =
        crate::hal::common::virt::teardown_lifecycle_code(VMX_LIFECYCLE.load(Ordering::Relaxed))
    {
        VMX_LIFECYCLE.store(code, Ordering::Relaxed);
    }

    if let Some(code) =
        crate::hal::common::virt::teardown_lifecycle_code(SVM_LIFECYCLE.load(Ordering::Relaxed))
    {
        SVM_LIFECYCLE.store(code, Ordering::Relaxed);
    }
}
