use crate::hal::common::virt::{
    VirtCaps, VirtEnableState, VirtStatus, BLOCKER_CODE_NO_HARDWARE, LIFECYCLE_CODE_FAILED,
    LIFECYCLE_CODE_PREPARED, LIFECYCLE_CODE_UNINITIALIZED,
};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
pub mod detail;
mod hooks;
mod ops;
mod regions;
mod support;
mod svm;
mod vmx;
use regions::*;
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

const VIRT_CAP_VMX: u32 = 1 << 0;
const VIRT_CAP_SVM: u32 = 1 << 1;
const VIRT_CAP_HYPERVISOR: u32 = 1 << 2;

const VIRT_ENABLE_VMX: u32 = 1 << 0;
const VIRT_ENABLE_VMXON: u32 = 1 << 1;
const VIRT_ENABLE_SVM: u32 = 1 << 2;

static VIRT_CAPS_BITS: AtomicU32 = AtomicU32::new(0);
static VIRT_ENABLE_BITS: AtomicU32 = AtomicU32::new(0);
static VIRT_VM_LAUNCH_READY: AtomicBool = AtomicBool::new(false);
static VIRT_VM_BLOCKER: AtomicU8 = AtomicU8::new(BLOCKER_CODE_NO_HARDWARE);
static VMX_VMCS_READY: AtomicBool = AtomicBool::new(false);
static SVM_VMCB_READY: AtomicBool = AtomicBool::new(false);
static VMX_LIFECYCLE: AtomicU8 = AtomicU8::new(LIFECYCLE_CODE_UNINITIALIZED);
static SVM_LIFECYCLE: AtomicU8 = AtomicU8::new(LIFECYCLE_CODE_UNINITIALIZED);
static VIRT_PREP_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static VIRT_PREP_SUCCESS: AtomicU64 = AtomicU64::new(0);
static VIRT_PREP_FAILURES: AtomicU64 = AtomicU64::new(0);

const IA32_FEATURE_CONTROL: u32 = 0x3A;
const IA32_EFER: u32 = 0xC000_0080;
const IA32_VMX_BASIC: u32 = 0x480;

fn prepare_vm_launch_structures(caps: VirtCaps, enabled: VirtEnableState) {
    let vmx_ok = if caps.vmx && enabled.vmx_enabled && enabled.vmxon_active {
        vmx::prepare_vmcs_region()
    } else {
        false
    };
    let svm_ok = if caps.svm && enabled.svm_enabled {
        svm::prepare_vmcb_region()
    } else {
        false
    };

    if !(caps.vmx && enabled.vmx_enabled && enabled.vmxon_active) {
        VMX_VMCS_READY.store(false, Ordering::Relaxed);
        VMX_LIFECYCLE.store(LIFECYCLE_CODE_UNINITIALIZED, Ordering::Relaxed);
    } else if !vmx_ok {
        VMX_LIFECYCLE.store(LIFECYCLE_CODE_FAILED, Ordering::Relaxed);
        crate::klog_warn!("VMCS preparation failed");
    } else {
        VMX_LIFECYCLE.store(LIFECYCLE_CODE_PREPARED, Ordering::Relaxed);
    }

    if !(caps.svm && enabled.svm_enabled) {
        SVM_VMCB_READY.store(false, Ordering::Relaxed);
        SVM_LIFECYCLE.store(LIFECYCLE_CODE_UNINITIALIZED, Ordering::Relaxed);
    } else if !svm_ok {
        SVM_LIFECYCLE.store(LIFECYCLE_CODE_FAILED, Ordering::Relaxed);
        crate::klog_warn!("VMCB preparation failed");
    } else {
        SVM_LIFECYCLE.store(LIFECYCLE_CODE_PREPARED, Ordering::Relaxed);
    }
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

pub fn detect_caps() -> VirtCaps {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::__cpuid;

        let leaf1 = __cpuid(0x1);
        let vmx = (leaf1.ecx & (1 << 5)) != 0;
        let hypervisor_present = (leaf1.ecx & (1 << 31)) != 0;

        let ext_leaf = __cpuid(0x8000_0000).eax;
        let svm = if ext_leaf >= 0x8000_0001 {
            let ext1 = __cpuid(0x8000_0001);
            (ext1.ecx & (1 << 2)) != 0
        } else {
            false
        };

        let caps = VirtCaps {
            vmx,
            svm,
            hypervisor_present,
        };
        let enabled = support::bits_to_enable(VIRT_ENABLE_BITS.load(Ordering::Relaxed));
        support::persist_status(caps, enabled);
        caps
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        VirtCaps::default()
    }
}

pub fn try_enable_hardware_virtualization() -> VirtEnableState {
    #[cfg(target_arch = "x86_64")]
    {
        let caps = detect_caps();
        let vmx_enabled = if caps.vmx {
            vmx::try_enable_vmx()
        } else {
            false
        };
        let vmxon_active = if vmx_enabled {
            vmx::try_enter_vmx_operation()
        } else {
            false
        };
        let svm_enabled = if caps.svm {
            svm::try_enable_svm()
        } else {
            false
        };
        let enabled = VirtEnableState {
            vmx_enabled,
            vmxon_active,
            svm_enabled,
        };
        prepare_vm_launch_structures(caps, enabled);
        support::persist_status(caps, enabled);
        enabled
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        VirtEnableState::default()
    }
}

#[cfg(test)]
mod tests {
    use super::support::{caps_to_bits, enable_to_bits};
    use super::{
        advanced_operations_profile, dirty_logging_ready, guest_control_profile,
        guest_exit_profile, guest_launch_profile, guest_lifecycle_profile, guest_operation_profile,
        guest_runtime_profile, live_migration_ready, snapshot_ready, SVM_LIFECYCLE, SVM_VMCB_READY,
        VIRT_CAPS_BITS, VIRT_ENABLE_BITS, VIRT_PREP_ATTEMPTS, VIRT_PREP_FAILURES,
        VIRT_PREP_SUCCESS, VIRT_VM_BLOCKER, VIRT_VM_LAUNCH_READY, VMX_LIFECYCLE, VMX_VMCS_READY,
    };
    use crate::hal::common::virt::{
        VirtCaps, VirtEnableState, BLOCKER_CODE_NONE, LIFECYCLE_CODE_ACTIVE,
        LIFECYCLE_CODE_UNINITIALIZED,
    };
    use core::sync::atomic::Ordering;

    #[test_case]
    fn advanced_operations_profile_reports_ready_vmX_path() {
        VIRT_CAPS_BITS.store(
            caps_to_bits(VirtCaps {
                vmx: true,
                svm: false,
                hypervisor_present: false,
            }),
            Ordering::Relaxed,
        );
        VIRT_ENABLE_BITS.store(
            enable_to_bits(VirtEnableState {
                vmx_enabled: true,
                vmxon_active: true,
                svm_enabled: false,
            }),
            Ordering::Relaxed,
        );
        VIRT_VM_LAUNCH_READY.store(true, Ordering::Relaxed);
        VIRT_VM_BLOCKER.store(BLOCKER_CODE_NONE, Ordering::Relaxed);
        VMX_VMCS_READY.store(true, Ordering::Relaxed);
        SVM_VMCB_READY.store(false, Ordering::Relaxed);
        VMX_LIFECYCLE.store(LIFECYCLE_CODE_ACTIVE, Ordering::Relaxed);
        SVM_LIFECYCLE.store(LIFECYCLE_CODE_UNINITIALIZED, Ordering::Relaxed);
        VIRT_PREP_ATTEMPTS.store(1, Ordering::Relaxed);
        VIRT_PREP_SUCCESS.store(1, Ordering::Relaxed);
        VIRT_PREP_FAILURES.store(0, Ordering::Relaxed);

        let _ = snapshot_ready();
        let _ = dirty_logging_ready();
        let _ = live_migration_ready();
        let profile = advanced_operations_profile();
        assert!(profile.0);
        assert_eq!(profile.3, "advanced");
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
        assert_eq!(exits.0, "guest-exit-ready");
        assert!(exits.1);
        assert!(exits.2);
        assert!(exits.3);
        assert!(exits.4);
        let launch = guest_launch_profile();
        assert_eq!(launch.0, "guest-launch-partial");
        assert!(launch.1);
        assert!(launch.2);
        let ops = guest_operation_profile();
        assert_eq!(ops.launch_stage, "guest-launch-partial");
        assert_eq!(ops.runtime_stage, "guest-control-ready");
        assert_eq!(ops.exit_stage, "guest-exit-ready");
        assert!(ops.control_ready);
        assert!(ops.trap_ready);
    }
}
