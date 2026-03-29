use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_massif_heap,
        &test_massif_stack,
    ]
}

fn test_massif_heap() -> TestResult {
    TestResult::pass("nightly::profiling::massif::heap")
}

fn test_massif_stack() -> TestResult {
    TestResult::pass("nightly::profiling::massif::stack")
}
