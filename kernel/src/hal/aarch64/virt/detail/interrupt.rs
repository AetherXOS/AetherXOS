use crate::hal::common::virt::{
    VirtStatus, INTERRUPT_BASIC, INTERRUPT_GICV3_READY, INTERRUPT_GIC_BASIC, INTERRUPT_NONE,
};

pub(super) fn interrupt_detail(
    status: VirtStatus,
    gic_initialized: bool,
    gic_version: u32,
) -> &'static str {
    if status.vm_launch_ready && gic_initialized && gic_version >= 3 {
        INTERRUPT_GICV3_READY
    } else if status.vm_launch_ready && gic_initialized {
        INTERRUPT_GIC_BASIC
    } else if status.caps.hypervisor_present {
        INTERRUPT_BASIC
    } else {
        INTERRUPT_NONE
    }
}

#[cfg(test)]
mod tests {
    use super::interrupt_detail;
    use crate::hal::common::virt::{VirtCaps, VirtEnableState, VirtStatus};

    fn el2_status() -> VirtStatus {
        VirtStatus {
            caps: VirtCaps {
                vmx: false,
                svm: false,
                hypervisor_present: true,
            },
            enabled: VirtEnableState::default(),
            vm_launch_ready: true,
            blocker: "none",
            vmx_vmcs_ready: false,
            svm_vmcb_ready: false,
            prep_attempts: 0,
            prep_success: 0,
            prep_failures: 0,
            vmx_lifecycle: "active",
            svm_lifecycle: "active",
        }
    }

    #[test_case]
    fn interrupt_detail_prefers_gicv3_path() {
        let status = el2_status();
        assert_eq!(interrupt_detail(status, true, 3), INTERRUPT_GICV3_READY);
        assert_eq!(interrupt_detail(status, true, 2), INTERRUPT_GIC_BASIC);
    }
}
