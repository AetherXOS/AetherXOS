use super::*;

#[test_case]
fn runtime_policy_snapshot_contract_matches_effective_virtualization_policy() {
    assert!(runtime_policy_snapshot_contract_holds(
        runtime_policy_snapshot()
    ));
}

#[test_case]
fn runtime_policy_contract_self_test_passes() {
    assert!(run_runtime_policy_contract_self_test().passed());
}
