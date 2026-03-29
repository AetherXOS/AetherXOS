use crate::hal::common::virt::{
    VirtStatus, BACKEND_EL2_ACTIVE, BACKEND_EL2_DETECTED, BACKEND_EL2_FULL,
};

pub(super) fn backend_detail(
    status: VirtStatus,
    gic_initialized: bool,
    timer_frequency_hz: u64,
) -> &'static str {
    if status.vm_launch_ready && gic_initialized && timer_frequency_hz != 0 {
        BACKEND_EL2_FULL
    } else if status.vm_launch_ready {
        BACKEND_EL2_ACTIVE
    } else {
        BACKEND_EL2_DETECTED
    }
}

pub(super) fn trap_handling_ready(
    status: VirtStatus,
    gic_initialized: bool,
    timer_frequency_hz: u64,
) -> bool {
    status.vm_launch_ready && gic_initialized && timer_frequency_hz != 0
}
