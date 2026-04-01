use super::*;

#[cfg(target_arch = "x86_64")]
pub(super) fn prepare_vmcs_region() -> bool {
    let basic = support::rdmsr(IA32_VMX_BASIC);
    let revision = (basic & 0x7fff_ffff) as u32;

    unsafe {
        VMCS_REGION.revision_id = revision;
        VMCS_REGION.abort_indicator = 0;
    }

    let vmcs_phys = support::virt_to_phys((&raw const VMCS_REGION) as usize);
    let ok = vmcs_phys.unwrap_or(0) != 0;
    VMX_VMCS_READY.store(ok, Ordering::Relaxed);
    support::set_prep_result(ok);
    ok
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn prepare_vmcs_region() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
pub(super) fn try_enable_vmx() -> bool {
    let mut feature = support::rdmsr(IA32_FEATURE_CONTROL);
    let locked = (feature & 0x1) != 0;
    let vmx_outside_smx = (feature & (1 << 2)) != 0;

    if !locked {
        feature |= 0x1;
        feature |= 1 << 2;
        support::wrmsr(IA32_FEATURE_CONTROL, feature);
    } else if !vmx_outside_smx {
        return false;
    }

    let mut cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, nomem));
    }
    cr4 |= 1 << 13;
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, nomem));
    }

    true
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn try_enable_vmx() -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
pub(super) fn try_enter_vmx_operation() -> bool {
    let basic = support::rdmsr(IA32_VMX_BASIC);
    let revision = (basic & 0x7fff_ffff) as u32;

    unsafe {
        VMXON_REGION.revision_id = revision;
    }

    let vmxon_phys = support::virt_to_phys((&raw const VMXON_REGION) as usize);
    let Some(vmxon_phys) = vmxon_phys else {
        crate::klog_warn!("VMXON skipped: could not translate VMXON region to physical address");
        return false;
    };

    let mut failed: u8;
    unsafe {
        core::arch::asm!(
            "vmxon [{ptr}]",
            "setna {failed}",
            ptr = in(reg) &vmxon_phys,
            failed = out(reg_byte) failed,
            options(nostack)
        );
    }

    failed == 0
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn try_enter_vmx_operation() -> bool {
    false
}
