use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_dhat_heap,
        &test_dhat_allocations,
    ]
}

fn test_dhat_heap() -> TestResult {
    TestResult::pass("nightly::profiling::dhat::heap")
}

fn test_dhat_allocations() -> TestResult {
    TestResult::pass("nightly::profiling::dhat::allocations")
}
