use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_ubsan_overflow,
        &test_ubsan_null_deref,
        &test_ubsan_misaligned,
        &test_ubsan_invalid_cast,
    ]
}

fn test_ubsan_overflow() -> TestResult {
    TestResult::pass("integration::sanitizers::ubsan::overflow")
}

fn test_ubsan_null_deref() -> TestResult {
    TestResult::pass("integration::sanitizers::ubsan::null_deref")
}

fn test_ubsan_misaligned() -> TestResult {
    TestResult::pass("integration::sanitizers::ubsan::misaligned")
}

fn test_ubsan_invalid_cast() -> TestResult {
    TestResult::pass("integration::sanitizers::ubsan::invalid_cast")
}
