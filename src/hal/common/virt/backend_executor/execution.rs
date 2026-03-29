use super::types::*;
use crate::hal::common::virt::{
    backend_family, GuestTransitionState, BACKEND_FAMILY_EL2, BACKEND_FAMILY_SVM,
    BACKEND_FAMILY_VMX, RUNTIME_PATH_BLOCKED, RUNTIME_PATH_EL2_ENTRY, RUNTIME_PATH_EL2_RESUME,
    RUNTIME_PATH_EL2_TRAP, RUNTIME_PATH_GENERIC, RUNTIME_PATH_SVM_ENTRY, RUNTIME_PATH_SVM_RESUME,
    RUNTIME_PATH_SVM_TRAP, RUNTIME_PATH_VMX_ENTRY, RUNTIME_PATH_VMX_RESUME, RUNTIME_PATH_VMX_TRAP,
};

#[inline(always)]
fn virtualization_execution_policy() -> VirtualizationExecutionPolicy {
    let policy = crate::config::KernelConfig::virtualization_policy_profile();
    VirtualizationExecutionPolicy {
        runtime: policy.runtime,
        cargo: policy.cargo,
        effective: policy.effective,
    }
}

impl VirtualizationExecutionPolicy {
    #[inline(always)]
    pub fn entry_scope(self) -> &'static str {
        crate::config::KernelConfig::virtualization_policy_scope_profile().entry
    }

    #[inline(always)]
    pub fn resume_scope(self) -> &'static str {
        crate::config::KernelConfig::virtualization_policy_scope_profile().resume
    }

    #[inline(always)]
    pub fn trap_dispatch_scope(self) -> &'static str {
        crate::config::KernelConfig::virtualization_policy_scope_profile().trap_dispatch
    }

    #[inline(always)]
    pub fn nested_scope(self) -> &'static str {
        crate::config::KernelConfig::virtualization_policy_scope_profile().nested
    }
}

impl GuestBackendExecution {
    #[inline(always)]
    pub fn entry_scope(self) -> &'static str {
        self.policy.entry_scope()
    }

    #[inline(always)]
    pub fn resume_scope(self) -> &'static str {
        self.policy.resume_scope()
    }

    #[inline(always)]
    pub fn trap_dispatch_scope(self) -> &'static str {
        self.policy.trap_dispatch_scope()
    }

    #[inline(always)]
    pub fn nested_scope(self) -> &'static str {
        self.policy.nested_scope()
    }

    #[inline(always)]
    pub fn time_virtualization_scope(self) -> &'static str {
        crate::config::KernelConfig::virtualization_policy_scope_profile().time_virtualization
    }
}

#[inline(always)]
pub fn backend_operational_path(
    backend_detail: &'static str,
    transition: GuestTransitionState,
) -> &'static str {
    match (
        backend_family(backend_detail),
        transition.selected_phase,
        transition.ready,
    ) {
        (BACKEND_FAMILY_VMX, "entry", true) => RUNTIME_PATH_VMX_ENTRY,
        (BACKEND_FAMILY_VMX, "resume", true) => RUNTIME_PATH_VMX_RESUME,
        (BACKEND_FAMILY_VMX, "trap", _) => RUNTIME_PATH_VMX_TRAP,
        (BACKEND_FAMILY_SVM, "entry", true) => RUNTIME_PATH_SVM_ENTRY,
        (BACKEND_FAMILY_SVM, "resume", true) => RUNTIME_PATH_SVM_RESUME,
        (BACKEND_FAMILY_SVM, "trap", _) => RUNTIME_PATH_SVM_TRAP,
        (BACKEND_FAMILY_EL2, "entry", true) => RUNTIME_PATH_EL2_ENTRY,
        (BACKEND_FAMILY_EL2, "resume", true) => RUNTIME_PATH_EL2_RESUME,
        (BACKEND_FAMILY_EL2, "trap", _) => RUNTIME_PATH_EL2_TRAP,
        (_, _, true) => RUNTIME_PATH_GENERIC,
        (_, _, false) => RUNTIME_PATH_BLOCKED,
    }
}

#[inline(always)]
pub fn guest_backend_execution(
    backend_detail: &'static str,
    capability_detail: &'static str,
    feature_detail: &'static str,
    transition: GuestTransitionState,
) -> GuestBackendExecution {
    GuestBackendExecution {
        backend_family: backend_family(backend_detail),
        backend_detail,
        capability_detail,
        feature_detail,
        transition_stage: transition.stage,
        selected_phase: transition.selected_phase,
        selected_action: transition.selected_action,
        operational_path: backend_operational_path(backend_detail, transition),
        ready: transition.ready,
        blocked_by: transition.blocked_by,
        policy: virtualization_execution_policy(),
    }
}
