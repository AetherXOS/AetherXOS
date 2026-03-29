pub const BLOCKER_CODE_NONE: u8 = 0;
pub const BLOCKER_CODE_NO_HARDWARE: u8 = 1;
pub const BLOCKER_CODE_RUNNING_UNDER_HV: u8 = 2;
pub const BLOCKER_CODE_VMX_NOT_ENABLED: u8 = 3;
pub const BLOCKER_CODE_VMXON_NOT_ACTIVE: u8 = 4;
pub const BLOCKER_CODE_SVM_NOT_ENABLED: u8 = 5;
pub const BLOCKER_CODE_VMCS_NOT_READY: u8 = 6;
pub const BLOCKER_CODE_VMCB_NOT_READY: u8 = 7;

pub const BLOCKER_CODE_NO_EL2: u8 = 1;
pub const BLOCKER_CODE_EL2_DISABLED: u8 = 2;

pub const BLOCKER_LABEL_NONE: &str = "none";
pub const BLOCKER_LABEL_NO_HARDWARE: &str = "no-vmx-or-svm";
pub const BLOCKER_LABEL_RUNNING_UNDER_HV: &str = "running-under-hypervisor";
pub const BLOCKER_LABEL_VMX_NOT_ENABLED: &str = "vmx-enable-failed";
pub const BLOCKER_LABEL_VMXON_NOT_ACTIVE: &str = "vmxon-failed";
pub const BLOCKER_LABEL_SVM_NOT_ENABLED: &str = "svm-enable-failed";
pub const BLOCKER_LABEL_VMCS_NOT_READY: &str = "vmcs-not-ready";
pub const BLOCKER_LABEL_VMCB_NOT_READY: &str = "vmcb-not-ready";
pub const BLOCKER_LABEL_EL2_NOT_SUPPORTED: &str = "EL2 Not Supported";
pub const BLOCKER_LABEL_EL2_NOT_ACTIVE: &str = "EL2 Not Active";

pub const LIFECYCLE_CODE_UNINITIALIZED: u8 = 0;
pub const LIFECYCLE_CODE_PREPARED: u8 = 1;
pub const LIFECYCLE_CODE_ACTIVE: u8 = 2;
pub const LIFECYCLE_CODE_TORN_DOWN: u8 = 3;
pub const LIFECYCLE_CODE_FAILED: u8 = 4;

pub const LIFECYCLE_STATE_UNINITIALIZED: &str = "uninitialized";
pub const LIFECYCLE_STATE_PREPARED: &str = "prepared";
pub const LIFECYCLE_STATE_ACTIVE: &str = "active";
pub const LIFECYCLE_STATE_TORN_DOWN: &str = "torn-down";
pub const LIFECYCLE_STATE_FAILED: &str = "failed";

pub const LIFECYCLE_SUMMARY_TRAP_READY: &str = "trap-ready";
pub const LIFECYCLE_SUMMARY_RESUME_READY: &str = "resume-ready";
pub const LIFECYCLE_SUMMARY_LAUNCH_READY: &str = "launch-ready";
pub const LIFECYCLE_SUMMARY_CAPABILITY_ACTIVE: &str = "capability-active";
pub const LIFECYCLE_SUMMARY_PREPARED: &str = "prepared";
pub const LIFECYCLE_SUMMARY_DETECTED: &str = "detected";
pub const LIFECYCLE_SUMMARY_BLOCKED: &str = "blocked";
pub const LIFECYCLE_SUMMARY_RESUME_POLICY_LIMITED: &str = "resume-policy-limited";
pub const LIFECYCLE_SUMMARY_PREPARED_POLICY_LIMITED: &str = "prepared-policy-limited";

#[allow(dead_code)]
#[inline(always)]
pub fn lifecycle_label(code: u8) -> &'static str {
    match code {
        LIFECYCLE_CODE_UNINITIALIZED => LIFECYCLE_STATE_UNINITIALIZED,
        LIFECYCLE_CODE_PREPARED => LIFECYCLE_STATE_PREPARED,
        LIFECYCLE_CODE_ACTIVE => LIFECYCLE_STATE_ACTIVE,
        LIFECYCLE_CODE_TORN_DOWN => LIFECYCLE_STATE_TORN_DOWN,
        LIFECYCLE_CODE_FAILED => LIFECYCLE_STATE_FAILED,
        _ => "unknown",
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn x86_blocker_label(code: u8) -> &'static str {
    match code {
        BLOCKER_CODE_NONE => BLOCKER_LABEL_NONE,
        BLOCKER_CODE_NO_HARDWARE => BLOCKER_LABEL_NO_HARDWARE,
        BLOCKER_CODE_RUNNING_UNDER_HV => BLOCKER_LABEL_RUNNING_UNDER_HV,
        BLOCKER_CODE_VMX_NOT_ENABLED => BLOCKER_LABEL_VMX_NOT_ENABLED,
        BLOCKER_CODE_VMXON_NOT_ACTIVE => BLOCKER_LABEL_VMXON_NOT_ACTIVE,
        BLOCKER_CODE_SVM_NOT_ENABLED => BLOCKER_LABEL_SVM_NOT_ENABLED,
        BLOCKER_CODE_VMCS_NOT_READY => BLOCKER_LABEL_VMCS_NOT_READY,
        BLOCKER_CODE_VMCB_NOT_READY => BLOCKER_LABEL_VMCB_NOT_READY,
        _ => "unknown",
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn el2_blocker_label(code: u8) -> &'static str {
    match code {
        BLOCKER_CODE_NONE => "None",
        BLOCKER_CODE_NO_EL2 => BLOCKER_LABEL_EL2_NOT_SUPPORTED,
        BLOCKER_CODE_EL2_DISABLED => BLOCKER_LABEL_EL2_NOT_ACTIVE,
        _ => "Unknown",
    }
}
