use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_creusot_contracts,
        &test_creusot_verification,
    ]
}

fn test_creusot_contracts() -> TestResult {
    TestResult::pass("nightly::verification::creusot::contracts")
}

fn test_creusot_verification() -> TestResult {
    TestResult::pass("nightly::verification::creusot::verification")
}
