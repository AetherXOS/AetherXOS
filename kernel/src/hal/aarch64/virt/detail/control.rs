use crate::hal::common::virt::{
    VirtStatus, CONTROL_EL2_ACTIVE, CONTROL_EL2_DETECTED, CONTROL_EL2_PREPARED, CONTROL_NONE,
};

pub(super) fn control_detail(
    status: VirtStatus,
    gic_initialized: bool,
    timer_frequency_hz: u64,
) -> &'static str {
    if status.vm_launch_ready && gic_initialized && timer_frequency_hz != 0 {
        CONTROL_EL2_ACTIVE
    } else if status.vm_launch_ready {
        CONTROL_EL2_PREPARED
    } else if status.caps.hypervisor_present {
        CONTROL_EL2_DETECTED
    } else {
        CONTROL_NONE
    }
}
