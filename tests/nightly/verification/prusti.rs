use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_prusti_contracts,
        &test_prusti_verification,
    ]
}

fn test_prusti_contracts() -> TestResult {
    TestResult::pass("nightly::verification::prusti::contracts")
}

fn test_prusti_verification() -> TestResult {
    TestResult::pass("nightly::verification::prusti::verification")
}
