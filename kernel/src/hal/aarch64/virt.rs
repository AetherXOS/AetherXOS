use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use crate::hal::common::virt::{
    VirtCaps, VirtEnableState, VirtStatus, BLOCKER_CODE_EL2_DISABLED, BLOCKER_CODE_NONE,
    BLOCKER_CODE_NO_EL2, LIFECYCLE_CODE_FAILED, LIFECYCLE_CODE_PREPARED,
    LIFECYCLE_CODE_UNINITIALIZED,
};

pub mod detail;
mod el2;
mod hooks;
mod ops;
mod support;
pub use hooks::{
    guest_backend_execution, guest_backend_runtime_plan, guest_backend_state_machine,
    guest_entry_intent, guest_entry_operation, guest_exit_hook, guest_launch_hook,
    guest_next_runtime_action, guest_operation_decision, guest_operation_hooks,
    guest_operation_plan, guest_resume_intent, guest_resume_operation, guest_runtime_dispatch_hint,
    guest_runtime_execution, guest_runtime_hook, guest_runtime_scheduling_profile,
    guest_transition_state, guest_trap_intent, guest_trap_operation,
};
pub use ops::{
    advanced_operations_profile, dirty_logging_ready, guest_control_profile, guest_exit_profile,
    guest_launch_profile, guest_lifecycle_profile, guest_operation_profile, guest_runtime_profile,
    live_migration_ready, snapshot_ready,
};

static HYP_ACTIVE: AtomicBool = AtomicBool::new(false);
static HYP_SUPPORTED: AtomicBool = AtomicBool::new(false);
static VM_LAUNCH_READY: AtomicBool = AtomicBool::new(false);
static VM_BLOCKER: AtomicU8 = AtomicU8::new(BLOCKER_CODE_NO_EL2);
static EL2_LIFECYCLE: AtomicU8 = AtomicU8::new(LIFECYCLE_CODE_UNINITIALIZED);
static PREP_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PREP_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PREP_FAILURES: AtomicU64 = AtomicU64::new(0);

fn persist_status(caps: VirtCaps, launch_ready: bool) {
    HYP_ACTIVE.store(caps.hypervisor_present, Ordering::Relaxed);
    HYP_SUPPORTED.store(caps.hypervisor_present, Ordering::Relaxed);
    VM_LAUNCH_READY.store(launch_ready, Ordering::Relaxed);
    VM_BLOCKER.store(
        if launch_ready {
            BLOCKER_CODE_NONE
        } else if el2::el2_supported() {
            BLOCKER_CODE_EL2_DISABLED
        } else {
            BLOCKER_CODE_NO_EL2
        },
        Ordering::Relaxed,
    );
}

pub fn detect_caps() -> VirtCaps {
    let caps = VirtCaps {
        vmx: false,
        svm: false,
        hypervisor_present: el2::el2_active() || el2::el2_supported(),
    };
    persist_status(caps, caps.hypervisor_present);
    caps
}

pub fn try_enable_hardware_virtualization() -> VirtEnableState {
    PREP_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let caps = detect_caps();
    let launch_ready = caps.hypervisor_present;
    if launch_ready {
        PREP_SUCCESS.fetch_add(1, Ordering::Relaxed);
        EL2_LIFECYCLE.store(LIFECYCLE_CODE_PREPARED, Ordering::Relaxed);
    } else {
        PREP_FAILURES.fetch_add(1, Ordering::Relaxed);
        EL2_LIFECYCLE.store(LIFECYCLE_CODE_FAILED, Ordering::Relaxed);
    }
    persist_status(caps, launch_ready);
    VirtEnableState::default()
}

pub fn initialize_launch_context() -> bool {
    support::initialize_launch_context()
}

pub fn reset_launch_context() -> bool {
    support::reset_launch_context()
}

pub fn teardown_launch_context() {
    support::teardown_launch_context()
}

pub fn status() -> VirtStatus {
    support::status()
}

#[cfg(test)]
mod tests {
    use super::{
        advanced_operations_profile, dirty_logging_ready, guest_control_profile,
        guest_exit_profile, guest_launch_profile, guest_lifecycle_profile, guest_operation_profile,
        guest_runtime_profile, live_migration_ready, snapshot_ready, BLOCKER_CODE_NONE,
        EL2_LIFECYCLE, HYP_ACTIVE, HYP_SUPPORTED, LIFECYCLE_CODE_ACTIVE, PREP_ATTEMPTS,
        PREP_FAILURES, PREP_SUCCESS, VM_BLOCKER, VM_LAUNCH_READY,
    };
    use core::sync::atomic::Ordering;

    #[test_case]
    fn advanced_operations_profile_reports_ready_el2_path() {
        HYP_ACTIVE.store(true, Ordering::Relaxed);
        HYP_SUPPORTED.store(true, Ordering::Relaxed);
        VM_LAUNCH_READY.store(true, Ordering::Relaxed);
        VM_BLOCKER.store(BLOCKER_CODE_NONE, Ordering::Relaxed);
        EL2_LIFECYCLE.store(LIFECYCLE_CODE_ACTIVE, Ordering::Relaxed);
        PREP_ATTEMPTS.store(1, Ordering::Relaxed);
        PREP_SUCCESS.store(1, Ordering::Relaxed);
        PREP_FAILURES.store(0, Ordering::Relaxed);

        let _ = snapshot_ready();
        let _ = dirty_logging_ready();
        let _ = live_migration_ready();
        let profile = advanced_operations_profile();
        assert!(profile.0);
        assert!(matches!(profile.3, "advanced" | "hypervisor-grade"));
        let lifecycle = guest_lifecycle_profile();
        assert_eq!(lifecycle.0, "active");
        assert!(lifecycle.1);
        let control = guest_control_profile();
        assert_eq!(control.0, "guest-control-ready");
        assert!(control.1);
        assert!(control.2);
        let runtime = guest_runtime_profile();
        assert_eq!(runtime.0, "guest-control-ready");
        assert!(runtime.1);
        assert!(runtime.2);
        assert!(runtime.3);
        assert!(runtime.4);
        let exits = guest_exit_profile();
        assert_eq!(exits.0, "guest-exit-partial");
        assert!(exits.1);
        assert!(exits.2);
        assert!(exits.3);
        assert!(!exits.4);
        let launch = guest_launch_profile();
        assert_eq!(launch.0, "guest-launch-partial");
        assert!(launch.1);
        assert!(launch.2);
        let ops = guest_operation_profile();
        assert_eq!(ops.launch_stage, "guest-launch-partial");
        assert_eq!(ops.runtime_stage, "guest-control-ready");
        assert_eq!(ops.exit_stage, "guest-exit-partial");
        assert!(ops.control_ready);
        assert!(ops.trap_ready);
    }
}
