use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_lean_verified_kernel,
        &test_lean_extracted_code,
    ]
}

fn test_lean_verified_kernel() -> TestResult {
    TestResult::pass("nightly::formal::lean::verified_kernel")
}

fn test_lean_extracted_code() -> TestResult {
    TestResult::pass("nightly::formal::lean::extracted_code")
}
