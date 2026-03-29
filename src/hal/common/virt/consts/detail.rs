pub const CAPABILITY_VMX_DETECTED: &str = "vmx:cpuid-only";
pub const CAPABILITY_VMX_ENABLED: &str = "vmx:msr+cr4";
pub const CAPABILITY_VMX_ACTIVE: &str = "vmx:entry+vmcs+assist";
pub const CAPABILITY_SVM_DETECTED: &str = "svm:cpuid-only";
pub const CAPABILITY_SVM_ENABLED: &str = "svm:efer";
pub const CAPABILITY_SVM_ACTIVE: &str = "svm:efer+vmcb";
pub const CAPABILITY_EL2_DETECTED: &str = "el2:present";
pub const CAPABILITY_EL2_ENABLED: &str = "el2:entry";
pub const CAPABILITY_EL2_ACTIVE: &str = "el2:timer+gic+entry";

pub const FEATURE_VMX_DETECTED: &str = "vmx:basic-discovery";
pub const FEATURE_VMX_ENABLED: &str = "vmx:entry-controls";
pub const FEATURE_VMX_ACTIVE: &str = "vmx:ept-like+exit-controls";
pub const FEATURE_SVM_DETECTED: &str = "svm:basic-discovery";
pub const FEATURE_SVM_ENABLED: &str = "svm:control-enable";
pub const FEATURE_SVM_ACTIVE: &str = "svm:npt-like+vmcb";
pub const FEATURE_EL2_DETECTED: &str = "el2:basic-discovery";
pub const FEATURE_EL2_ENABLED: &str = "el2:entry-controls";
pub const FEATURE_EL2_ACTIVE: &str = "el2:vgic+vtimer";

pub const CONTROL_NONE: &str = "control-none";
pub const CONTROL_DETECTED: &str = "control-detected";
pub const CONTROL_VMX_ACTIVE: &str = "vmx-control-active";
pub const CONTROL_VMX_ENABLED: &str = "vmx-control-enabled";
pub const CONTROL_SVM_ENABLED: &str = "svm-control-enabled";
pub const CONTROL_EL2_ACTIVE: &str = "el2-control-active";
pub const CONTROL_EL2_PREPARED: &str = "el2-control-prepared";
pub const CONTROL_EL2_DETECTED: &str = "el2-detected";

pub const TRAP_NOT_READY: &str = "trap-not-ready";
pub const TRAP_STRUCTURES_READY: &str = "trap-structures-ready";
pub const TRAP_VMX_READY: &str = "vmx-traps-ready";
pub const TRAP_SVM_READY: &str = "svm-traps-ready";
pub const TRAP_EL2_READY: &str = "el2-traps-ready";
pub const TRAP_EL2_PARTIAL: &str = "el2-traps-partial";

pub const INTERRUPT_NONE: &str = "interrupt-none";
pub const INTERRUPT_BASIC: &str = "interrupt-basic";
pub const INTERRUPT_VMX_READY: &str = "vmx-posted-interrupt-ready";
pub const INTERRUPT_SVM_READY: &str = "svm-exit-interrupt-ready";
pub const INTERRUPT_GICV3_READY: &str = "gicv3-virt-ready";
pub const INTERRUPT_GIC_BASIC: &str = "gic-virt-basic";

pub const TIME_NONE: &str = "time-none";
pub const TIME_BASIC: &str = "time-basic";
pub const TIME_VMX_READY: &str = "vmx-tsc-offset-ready";
pub const TIME_SVM_READY: &str = "svm-tsc-offset-ready";
pub const TIME_CNTV_READY: &str = "cntv-virtual-time-ready";

pub const GUEST_CONTROL_READY: &str = "guest-control-ready";
pub const GUEST_CONTROL_PARTIAL: &str = "guest-control-partial";
pub const GUEST_CONTROL_PREPARED: &str = "guest-control-prepared";
pub const GUEST_CONTROL_BLOCKED: &str = "guest-control-blocked";

pub const GUEST_EXIT_READY: &str = "guest-exit-ready";
pub const GUEST_EXIT_PARTIAL: &str = "guest-exit-partial";
pub const GUEST_EXIT_PREPARED: &str = "guest-exit-prepared";
pub const GUEST_EXIT_BLOCKED: &str = "guest-exit-blocked";

pub const GUEST_LAUNCH_READY: &str = "guest-launch-ready";
pub const GUEST_LAUNCH_PARTIAL: &str = "guest-launch-partial";
pub const GUEST_LAUNCH_PREPARED: &str = "guest-launch-prepared";
pub const GUEST_LAUNCH_BLOCKED: &str = "guest-launch-blocked";

#[inline(always)]
pub fn guest_exit_requires_time_virtualization(exit_stage: &'static str) -> bool {
    exit_stage != GUEST_EXIT_READY
}

#[inline(always)]
pub fn guest_preferred_next_action(
    launch_allowed: bool,
    launch_action: &'static str,
    runtime_allowed: bool,
    runtime_action: &'static str,
    exit_action: &'static str,
) -> &'static str {
    if launch_allowed {
        launch_action
    } else if runtime_allowed {
        runtime_action
    } else {
        exit_action
    }
}
