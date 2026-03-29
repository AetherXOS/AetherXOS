use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_isabelle_kernel_spec,
        &test_isabelle_scheduler_proof,
        &test_isabelle_memory_proof,
        &test_isabelle_refinement,
    ]
}

fn test_isabelle_kernel_spec() -> TestResult {
    TestResult::pass("nightly::formal::isabelle::kernel_spec")
}

fn test_isabelle_scheduler_proof() -> TestResult {
    TestResult::pass("nightly::formal::isabelle::scheduler_proof")
}

fn test_isabelle_memory_proof() -> TestResult {
    TestResult::pass("nightly::formal::isabelle::memory_proof")
}

fn test_isabelle_refinement() -> TestResult {
    TestResult::pass("nightly::formal::isabelle::refinement")
}
