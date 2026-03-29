use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_honggfuzz_instrumentation,
        &test_honggfuzz_corpus,
    ]
}

fn test_honggfuzz_instrumentation() -> TestResult {
    TestResult::pass("integration::fuzzing::honggfuzz::instrumentation")
}

fn test_honggfuzz_corpus() -> TestResult {
    TestResult::pass("integration::fuzzing::honggfuzz::corpus")
}
