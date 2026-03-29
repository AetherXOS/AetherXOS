use super::*;

pub(crate) fn snapshot_ready() -> bool {
    let current = status();
    let monitoring_ready = current.vm_launch_ready;
    let state_save_restore_ready = current.vm_launch_ready;
    crate::hal::common::virt::snapshot_ready_from_flags(state_save_restore_ready, monitoring_ready)
}

pub(crate) fn dirty_logging_ready() -> bool {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let trap_handling_ready = current.vm_launch_ready && gic.initialized;
    let memory_isolation_ready =
        current.vm_launch_ready && gic.initialized && crate::hal::aarch64::dtb_addr().is_some();
    crate::hal::common::virt::dirty_logging_ready_from_flags(
        trap_handling_ready,
        memory_isolation_ready,
    )
}

pub(crate) fn live_migration_ready() -> bool {
    let timer = crate::hal::aarch64::timer::GenericTimer::stats();
    crate::hal::common::virt::live_migration_ready_from_flags(
        snapshot_ready(),
        timer.frequency_hz != 0,
        dirty_logging_ready(),
    )
}

pub(crate) fn advanced_operations_profile() -> (bool, bool, bool, &'static str) {
    crate::hal::common::virt::advanced_operations_profile(
        snapshot_ready(),
        dirty_logging_ready(),
        live_migration_ready(),
    )
}

pub(crate) fn guest_lifecycle_profile() -> (&'static str, bool, bool, &'static str) {
    let current = status();
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    let advanced = advanced_operations_profile();
    crate::hal::common::virt::guest_lifecycle_profile(
        lifecycle,
        current.vm_launch_ready,
        crate::hal::common::virt::guest_resume_ready(
            lifecycle,
            current.vm_launch_ready,
            crate::hal::common::virt::has_launch_context(lifecycle),
        ),
        advanced.3,
    )
}

pub(crate) fn guest_control_profile() -> (&'static str, bool, bool, bool) {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let control_ready = current.vm_launch_ready;
    let trap_ready = current.vm_launch_ready && gic.initialized;
    crate::hal::common::virt::guest_control_profile(
        control_ready,
        trap_ready,
        snapshot_ready(),
        current.vm_launch_ready,
    )
}

pub(crate) fn guest_runtime_profile() -> (&'static str, bool, bool, bool, bool) {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    crate::hal::common::virt::guest_runtime_profile(crate::hal::common::virt::GuestRuntimeFlags {
        launch_ready: current.vm_launch_ready,
        control_ready: current.vm_launch_ready,
        trap_ready: current.vm_launch_ready && gic.initialized,
        resume_ready: crate::hal::common::virt::guest_resume_ready(
            lifecycle,
            current.vm_launch_ready,
            crate::hal::common::virt::has_launch_context(lifecycle),
        ),
        snapshot_ready: snapshot_ready(),
    })
}

pub(crate) fn guest_exit_profile() -> (&'static str, bool, bool, bool, bool) {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let timer = crate::hal::aarch64::timer::GenericTimer::stats();
    let trap_ready = current.vm_launch_ready && gic.initialized;
    crate::hal::common::virt::guest_exit_profile(crate::hal::common::virt::GuestExitFlags {
        launch_ready: current.vm_launch_ready,
        trap_ready,
        trace_ready: trap_ready,
        interrupt_ready: current.vm_launch_ready && gic.initialized,
        time_ready: current.vm_launch_ready && timer.frequency_hz != 0,
    })
}

pub(crate) fn guest_launch_profile() -> (&'static str, bool, bool, bool) {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let memory_isolation_ready =
        current.vm_launch_ready && gic.initialized && crate::hal::aarch64::dtb_addr().is_some();
    crate::hal::common::virt::guest_launch_profile(crate::hal::common::virt::GuestLaunchFlags {
        launch_ready: current.vm_launch_ready,
        control_ready: current.vm_launch_ready,
        guest_entry_ready: current.vm_launch_ready && gic.initialized,
        memory_isolation_ready,
    })
}

pub(crate) fn guest_operation_profile() -> crate::hal::common::virt::GuestOperationProfile {
    let current = status();
    let gic = crate::hal::aarch64::gic::stats();
    let timer = crate::hal::aarch64::timer::GenericTimer::stats();
    let trap_ready = current.vm_launch_ready && gic.initialized;
    let memory_isolation_ready =
        current.vm_launch_ready && gic.initialized && crate::hal::aarch64::dtb_addr().is_some();
    let lifecycle =
        crate::hal::common::virt::primary_lifecycle(current.vmx_lifecycle, current.svm_lifecycle);
    crate::hal::common::virt::guest_operation_profile(
        crate::hal::common::virt::GuestLaunchFlags {
            launch_ready: current.vm_launch_ready,
            control_ready: current.vm_launch_ready,
            guest_entry_ready: current.vm_launch_ready && gic.initialized,
            memory_isolation_ready,
        },
        crate::hal::common::virt::GuestRuntimeFlags {
            launch_ready: current.vm_launch_ready,
            control_ready: current.vm_launch_ready,
            trap_ready,
            resume_ready: crate::hal::common::virt::guest_resume_ready(
                lifecycle,
                current.vm_launch_ready,
                crate::hal::common::virt::has_launch_context(lifecycle),
            ),
            snapshot_ready: snapshot_ready(),
        },
        crate::hal::common::virt::GuestExitFlags {
            launch_ready: current.vm_launch_ready,
            trap_ready,
            trace_ready: trap_ready,
            interrupt_ready: current.vm_launch_ready && gic.initialized,
            time_ready: current.vm_launch_ready && timer.frequency_hz != 0,
        },
    )
}
