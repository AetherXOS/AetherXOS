use super::*;

#[cfg(target_arch = "x86_64")]
pub(super) fn prepare_vmcb_region() -> bool {
    let vmcb_phys = support::virt_to_phys(vmcb_region_ptr() as usize);
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
    use crate::kernel::bit_utils::x86_64_arch::efer;
    let mut efer_val = support::rdmsr(IA32_EFER);
    efer_val = efer::SVME.set_bit(efer_val, true);
    support::wrmsr(IA32_EFER, efer_val);
    true
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn try_enable_svm() -> bool {
    false
}
