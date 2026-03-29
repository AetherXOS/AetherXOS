use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_perf_events,
        &test_perf_counters,
    ]
}

fn test_perf_events() -> TestResult {
    TestResult::pass("nightly::profiling::perf::events")
}

fn test_perf_counters() -> TestResult {
    TestResult::pass("nightly::profiling::perf::counters")
}
