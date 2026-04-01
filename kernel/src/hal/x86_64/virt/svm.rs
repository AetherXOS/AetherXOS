use super::*;

#[cfg(target_arch = "x86_64")]
pub(super) fn prepare_vmcb_region() -> bool {
    let vmcb_phys = support::virt_to_phys((&raw const VMCB_REGION) as usize);
    let ok = vmcb_phys.unwrap_or(0) != 0;
    SVM_VMCB_READY.store(ok, Ordering::Relaxed);
    support::set_prep_result(ok);
    ok
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn prepare_vmcb_region() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
pub(super) fn try_enable_svm() -> bool {
    let mut efer = support::rdmsr(IA32_EFER);
    efer |= 1 << 12;
    support::wrmsr(IA32_EFER, efer);
    true
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn try_enable_svm() -> bool {
    false
}
