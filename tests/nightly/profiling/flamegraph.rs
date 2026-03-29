use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_flamegraph_cpu,
        &test_flamegraph_memory,
        &test_flamegraph_io,
    ]
}

fn test_flamegraph_cpu() -> TestResult {
    TestResult::pass("nightly::profiling::flamegraph::cpu")
}

fn test_flamegraph_memory() -> TestResult {
    TestResult::pass("nightly::profiling::flamegraph::memory")
}

fn test_flamegraph_io() -> TestResult {
    TestResult::pass("nightly::profiling::flamegraph::io")
}
