use super::*;

#[test_case]
fn status_mapping_helpers_match_expected_profiles() {
    assert!(execution_profile_matches_status(
        crate::config::VirtualizationExecutionClass::LatencyCritical,
        "LatencyCritical",
    ));
    assert_eq!(
        expected_runtime_governor_class(crate::config::VirtualizationGovernorClass::Efficiency),
        crate::hal::common::virt::GOVERNOR_CLASS_EFFICIENCY
    );
}
