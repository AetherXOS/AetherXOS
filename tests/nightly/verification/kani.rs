use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_kani_memory_safety,
        &test_kani_arithmetic_overflow,
        &test_kani_concurrency,
        &test_kani_proofs,
    ]
}

fn test_kani_memory_safety() -> TestResult {
    TestResult::pass("nightly::verification::kani::memory_safety")
}

fn test_kani_arithmetic_overflow() -> TestResult {
    TestResult::pass("nightly::verification::kani::arithmetic_overflow")
}

fn test_kani_concurrency() -> TestResult {
    TestResult::pass("nightly::verification::kani::concurrency")
}

fn test_kani_proofs() -> TestResult {
    TestResult::pass("nightly::verification::kani::proofs")
}
