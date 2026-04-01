use super::{full_entry_enabled, full_resume_enabled, full_trap_enabled};
use crate::hal::common::virt::{
    GuestBackendExecution, RUNTIME_PATH_VMX_ENTRY, RUNTIME_PATH_VMX_RESUME, RUNTIME_PATH_VMX_TRAP,
    RUNTIME_STEP_DISPATCH_VMX_TRAP, RUNTIME_STEP_DISPATCH_VMX_TRAP_BASIC,
    RUNTIME_STEP_PREPARE_VMCS_ENTRY, RUNTIME_STEP_PREPARE_VMCS_ENTRY_BASIC,
    RUNTIME_STEP_RESUME_VMX_VCPU, RUNTIME_STEP_RESUME_VMX_VCPU_BASIC,
};

#[inline(always)]
fn detail_step(execution: GuestBackendExecution) -> Option<&'static str> {
    if execution.backend_family != "vmx" {
        return None;
    }
    if execution.selected_phase == "entry" && execution.capability_detail == "vmx:entry+vmcs+assist"
    {
        return Some(if full_entry_enabled(execution) {
            RUNTIME_STEP_PREPARE_VMCS_ENTRY
        } else {
            RUNTIME_STEP_PREPARE_VMCS_ENTRY_BASIC
        });
    }
    if execution.selected_phase == "resume"
        && execution.feature_detail == "vmx:ept-like+exit-controls"
    {
        return Some(if full_resume_enabled(execution) {
            RUNTIME_STEP_RESUME_VMX_VCPU
        } else {
            RUNTIME_STEP_RESUME_VMX_VCPU_BASIC
        });
    }
    if execution.selected_phase == "trap"
        && execution.feature_detail == "vmx:ept-like+exit-controls"
    {
        return Some(if full_trap_enabled(execution) {
            RUNTIME_STEP_DISPATCH_VMX_TRAP
        } else {
            RUNTIME_STEP_DISPATCH_VMX_TRAP_BASIC
        });
    }
    None
}

#[inline(always)]
fn fallback_step(path: &'static str) -> Option<&'static str> {
    match path {
        RUNTIME_PATH_VMX_ENTRY => Some(RUNTIME_STEP_PREPARE_VMCS_ENTRY),
        RUNTIME_PATH_VMX_RESUME => Some(RUNTIME_STEP_RESUME_VMX_VCPU),
        RUNTIME_PATH_VMX_TRAP => Some(RUNTIME_STEP_DISPATCH_VMX_TRAP),
        _ => None,
    }
}

#[inline(always)]
pub fn runtime_step(execution: GuestBackendExecution) -> Option<&'static str> {
    detail_step(execution).or_else(|| fallback_step(execution.operational_path))
}
