use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_libfuzzer_instrumentation,
        &test_libfuzzer_corpus,
    ]
}

fn test_libfuzzer_instrumentation() -> TestResult {
    TestResult::pass("integration::fuzzing::libfuzzer::instrumentation")
}

fn test_libfuzzer_corpus() -> TestResult {
    TestResult::pass("integration::fuzzing::libfuzzer::corpus")
}
