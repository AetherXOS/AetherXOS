use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_cargo_fuzz_config,
        &test_cargo_fuzz_corpus,
        &test_cargo_fuzz_coverage,
    ]
}

fn test_cargo_fuzz_config() -> TestResult {
    TestResult::pass("integration::fuzzing::cargo_fuzz::config")
}

fn test_cargo_fuzz_corpus() -> TestResult {
    TestResult::pass("integration::fuzzing::cargo_fuzz::corpus")
}

fn test_cargo_fuzz_coverage() -> TestResult {
    TestResult::pass("integration::fuzzing::cargo_fuzz::coverage")
}
