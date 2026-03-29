use crate::hal::common::virt::{
    VirtStatus, BACKEND_VMX_ACTIVE, BACKEND_VMX_DETECTED, BACKEND_VMX_ENABLED,
};

pub(super) fn backend_detail(status: VirtStatus) -> &'static str {
    if status.enabled.vmxon_active && status.vmx_vmcs_ready {
        BACKEND_VMX_ACTIVE
    } else if status.enabled.vmx_enabled {
        BACKEND_VMX_ENABLED
    } else {
        BACKEND_VMX_DETECTED
    }
}

pub(super) fn trap_handling_ready(status: VirtStatus) -> bool {
    status.enabled.vmxon_active && status.vmx_vmcs_ready
}
