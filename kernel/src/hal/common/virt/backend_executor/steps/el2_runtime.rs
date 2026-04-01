use super::{full_entry_enabled, full_resume_enabled, full_trap_enabled};
use crate::hal::common::virt::{
    GuestBackendExecution, RUNTIME_PATH_EL2_ENTRY, RUNTIME_PATH_EL2_RESUME, RUNTIME_PATH_EL2_TRAP,
    RUNTIME_STEP_DISPATCH_EL2_TRAP, RUNTIME_STEP_DISPATCH_EL2_TRAP_BASIC,
    RUNTIME_STEP_PREPARE_EL2_ENTRY, RUNTIME_STEP_PREPARE_EL2_ENTRY_BASIC,
    RUNTIME_STEP_RESUME_EL2_GUEST, RUNTIME_STEP_RESUME_EL2_GUEST_BASIC,
};

#[inline(always)]
fn detail_step(execution: GuestBackendExecution) -> Option<&'static str> {
    if execution.backend_family != "el2" {
        return None;
    }
    if execution.selected_phase == "entry" && execution.capability_detail == "el2:timer+gic+entry" {
        return Some(if full_entry_enabled(execution) {
            RUNTIME_STEP_PREPARE_EL2_ENTRY
        } else {
            RUNTIME_STEP_PREPARE_EL2_ENTRY_BASIC
        });
    }
    if execution.selected_phase == "resume" && execution.feature_detail == "el2:vgic+vtimer" {
        return Some(if full_resume_enabled(execution) {
            RUNTIME_STEP_RESUME_EL2_GUEST
        } else {
            RUNTIME_STEP_RESUME_EL2_GUEST_BASIC
        });
    }
    if execution.selected_phase == "trap" && execution.feature_detail == "el2:vgic+vtimer" {
        return Some(if full_trap_enabled(execution) {
            RUNTIME_STEP_DISPATCH_EL2_TRAP
        } else {
            RUNTIME_STEP_DISPATCH_EL2_TRAP_BASIC
        });
    }
    None
}

#[inline(always)]
fn fallback_step(path: &'static str) -> Option<&'static str> {
    match path {
        RUNTIME_PATH_EL2_ENTRY => Some(RUNTIME_STEP_PREPARE_EL2_ENTRY),
        RUNTIME_PATH_EL2_RESUME => Some(RUNTIME_STEP_RESUME_EL2_GUEST),
        RUNTIME_PATH_EL2_TRAP => Some(RUNTIME_STEP_DISPATCH_EL2_TRAP),
        _ => None,
    }
}

#[inline(always)]
pub fn runtime_step(execution: GuestBackendExecution) -> Option<&'static str> {
    detail_step(execution).or_else(|| fallback_step(execution.operational_path))
}
