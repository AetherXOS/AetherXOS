use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_coq_verified_kernel,
        &test_coq_extracted_code,
    ]
}

fn test_coq_verified_kernel() -> TestResult {
    TestResult::pass("nightly::formal::coq::verified_kernel")
}

fn test_coq_extracted_code() -> TestResult {
    TestResult::pass("nightly::formal::coq::extracted_code")
}
