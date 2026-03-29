use super::*;

pub(crate) fn blocker_reason(code: u8) -> &'static str {
    crate::hal::common::virt::x86_blocker_label(code)
}

pub(crate) fn lifecycle_reason(code: u8) -> &'static str {
    crate::hal::common::virt::lifecycle_label(code)
}

pub(crate) fn caps_to_bits(caps: VirtCaps) -> u32 {
    let mut bits = 0u32;
    if caps.vmx {
        bits |= VIRT_CAP_VMX;
    }
    if caps.svm {
        bits |= VIRT_CAP_SVM;
    }
    if caps.hypervisor_present {
        bits |= VIRT_CAP_HYPERVISOR;
    }
    bits
}

pub(crate) fn enable_to_bits(enabled: VirtEnableState) -> u32 {
    let mut bits = 0u32;
    if enabled.vmx_enabled {
        bits |= VIRT_ENABLE_VMX;
    }
    if enabled.vmxon_active {
        bits |= VIRT_ENABLE_VMXON;
    }
    if enabled.svm_enabled {
        bits |= VIRT_ENABLE_SVM;
    }
    bits
}

pub(crate) fn bits_to_caps(bits: u32) -> VirtCaps {
    VirtCaps {
        vmx: (bits & VIRT_CAP_VMX) != 0,
        svm: (bits & VIRT_CAP_SVM) != 0,
        hypervisor_present: (bits & VIRT_CAP_HYPERVISOR) != 0,
    }
}

pub(crate) fn bits_to_enable(bits: u32) -> VirtEnableState {
    VirtEnableState {
        vmx_enabled: (bits & VIRT_ENABLE_VMX) != 0,
        vmxon_active: (bits & VIRT_ENABLE_VMXON) != 0,
        svm_enabled: (bits & VIRT_ENABLE_SVM) != 0,
    }
}

pub(crate) fn set_prep_result(ok: bool) {
    VIRT_PREP_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if ok {
        VIRT_PREP_SUCCESS.fetch_add(1, Ordering::Relaxed);
    } else {
        VIRT_PREP_FAILURES.fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) fn evaluate_launch_readiness(caps: VirtCaps, enabled: VirtEnableState) -> (bool, u8) {
    if caps.hypervisor_present {
        return (
            false,
            crate::hal::common::virt::BLOCKER_CODE_RUNNING_UNDER_HV,
        );
    }

    if caps.vmx {
        if !enabled.vmx_enabled {
            return (
                false,
                crate::hal::common::virt::BLOCKER_CODE_VMX_NOT_ENABLED,
            );
        }
        if !enabled.vmxon_active {
            return (
                false,
                crate::hal::common::virt::BLOCKER_CODE_VMXON_NOT_ACTIVE,
            );
        }
        if !VMX_VMCS_READY.load(Ordering::Relaxed) {
            return (false, crate::hal::common::virt::BLOCKER_CODE_VMCS_NOT_READY);
        }
        return (true, crate::hal::common::virt::BLOCKER_CODE_NONE);
    }

    if caps.svm {
        if !enabled.svm_enabled {
            return (
                false,
                crate::hal::common::virt::BLOCKER_CODE_SVM_NOT_ENABLED,
            );
        }
        if !SVM_VMCB_READY.load(Ordering::Relaxed) {
            return (false, crate::hal::common::virt::BLOCKER_CODE_VMCB_NOT_READY);
        }
        return (true, crate::hal::common::virt::BLOCKER_CODE_NONE);
    }

    (false, crate::hal::common::virt::BLOCKER_CODE_NO_HARDWARE)
}

pub(crate) fn persist_status(caps: VirtCaps, enabled: VirtEnableState) {
    VIRT_CAPS_BITS.store(caps_to_bits(caps), Ordering::Relaxed);
    VIRT_ENABLE_BITS.store(enable_to_bits(enabled), Ordering::Relaxed);

    let (ready, blocker) = evaluate_launch_readiness(caps, enabled);
    VIRT_VM_LAUNCH_READY.store(ready, Ordering::Relaxed);
    VIRT_VM_BLOCKER.store(blocker, Ordering::Relaxed);
}
