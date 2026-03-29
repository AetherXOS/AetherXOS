use super::*;
use crate::hal::common::virt::{
    BACKEND_MODE_BASIC, BACKEND_MODE_BLOCKED, BACKEND_MODE_FULL, OPERATION_CLASS_BASIC,
    OPERATION_CLASS_BLOCKED, OPERATION_CLASS_FULL,
};

#[test_case]
fn runtime_mode_contract_accepts_consistent_states() {
    assert!(virtualization_runtime_mode_contract_holds(
        BACKEND_MODE_BLOCKED,
        OPERATION_CLASS_BLOCKED,
        Some("entry-policy-disabled"),
        None,
    ));
    assert!(virtualization_runtime_mode_contract_holds(
        BACKEND_MODE_BASIC,
        OPERATION_CLASS_BASIC,
        None,
        Some("live-migration-policy-disabled"),
    ));
    assert!(virtualization_runtime_mode_contract_holds(
        BACKEND_MODE_FULL,
        OPERATION_CLASS_FULL,
        None,
        None,
    ));
}

#[test_case]
fn runtime_mode_contract_rejects_inconsistent_states() {
    assert!(!virtualization_runtime_mode_contract_holds(
        BACKEND_MODE_BLOCKED,
        OPERATION_CLASS_FULL,
        Some("entry-policy-disabled"),
        None,
    ));
    assert!(!virtualization_runtime_mode_contract_holds(
        BACKEND_MODE_BASIC,
        OPERATION_CLASS_BASIC,
        Some("entry-policy-disabled"),
        Some("live-migration-policy-disabled"),
    ));
}
