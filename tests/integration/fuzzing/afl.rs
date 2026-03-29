use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_afl_instrumentation,
        &test_afl_corpus,
    ]
}

fn test_afl_instrumentation() -> TestResult {
    TestResult::pass("integration::fuzzing::afl::instrumentation")
}

fn test_afl_corpus() -> TestResult {
    TestResult::pass("integration::fuzzing::afl::corpus")
}
