use crate::hal::common::virt::{
    BACKEND_MODE_BASIC, BACKEND_MODE_BLOCKED, BACKEND_MODE_FULL, BACKEND_MODE_NONE,
    OPERATION_CLASS_BASIC, OPERATION_CLASS_BLOCKED, OPERATION_CLASS_FULL,
};

#[inline(always)]
pub fn virtualization_runtime_mode_contract_holds(
    selected_mode: &'static str,
    operation_class: &'static str,
    blocked_by: Option<&'static str>,
    policy_limited_by: Option<&'static str>,
) -> bool {
    match (selected_mode, operation_class) {
        (BACKEND_MODE_BLOCKED, OPERATION_CLASS_BLOCKED) => blocked_by.is_some(),
        (BACKEND_MODE_BASIC, OPERATION_CLASS_BASIC) => {
            blocked_by.is_none() && policy_limited_by.is_some()
        }
        (BACKEND_MODE_FULL, OPERATION_CLASS_FULL) => {
            blocked_by.is_none() && policy_limited_by.is_none()
        }
        (BACKEND_MODE_NONE, OPERATION_CLASS_BLOCKED) => blocked_by.is_some(),
        _ => false,
    }
}
