use crate::hal::common::virt::{
    VirtStatus, STAGE_GUEST_RUNNABLE, STAGE_HARDWARE_ENABLED, STAGE_LAUNCH_PREPARED,
    STAGE_UNAVAILABLE,
};

pub(super) fn launch_stage(status: VirtStatus, trap_handling_ready: bool) -> &'static str {
    if trap_handling_ready {
        STAGE_GUEST_RUNNABLE
    } else if status.vm_launch_ready {
        STAGE_LAUNCH_PREPARED
    } else if status.caps.hypervisor_present {
        STAGE_HARDWARE_ENABLED
    } else {
        STAGE_UNAVAILABLE
    }
}
