use crate::hal::common::virt::{VirtStatus, TRAP_EL2_PARTIAL, TRAP_EL2_READY, TRAP_NOT_READY};

pub(super) fn trap_detail(
    status: VirtStatus,
    trap_handling_ready: bool,
    gic_initialized: bool,
) -> &'static str {
    if trap_handling_ready {
        TRAP_EL2_READY
    } else if status.vm_launch_ready && gic_initialized {
        TRAP_EL2_PARTIAL
    } else {
        TRAP_NOT_READY
    }
}
