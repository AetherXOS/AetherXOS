#[test_case]
fn virtualization_effective_execution_contract_matches_enablement() {
    assert!(
        crate::kernel::virtualization_contract::virtualization_effective_execution_contract_holds()
    );
}

#[test_case]
fn virtualization_effective_governor_contract_matches_enablement() {
    assert!(
        crate::kernel::virtualization_contract::virtualization_effective_governor_contract_holds()
    );
}
