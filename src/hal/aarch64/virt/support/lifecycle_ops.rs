use super::*;

pub(crate) fn initialize_launch_context() -> bool {
    let current = status();
    if let Some(code) = crate::hal::common::virt::activate_lifecycle_code(
        current.vm_launch_ready,
        current.vm_launch_ready,
    ) {
        EL2_LIFECYCLE.store(code, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub(crate) fn reset_launch_context() -> bool {
    if let Some(code) =
        crate::hal::common::virt::reset_lifecycle_code(VM_LAUNCH_READY.load(Ordering::Relaxed))
    {
        EL2_LIFECYCLE.store(code, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub(crate) fn teardown_launch_context() {
    if let Some(code) =
        crate::hal::common::virt::teardown_lifecycle_code(EL2_LIFECYCLE.load(Ordering::Relaxed))
    {
        EL2_LIFECYCLE.store(code, Ordering::Relaxed);
    }
}
