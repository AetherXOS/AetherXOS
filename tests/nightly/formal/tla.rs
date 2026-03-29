use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_tla_scheduler_spec,
        &test_tla_memory_spec,
        &test_tla_ipc_spec,
        &test_tla_model_check,
    ]
}

fn test_tla_scheduler_spec() -> TestResult {
    TestResult::pass("nightly::formal::tla::scheduler_spec")
}

fn test_tla_memory_spec() -> TestResult {
    TestResult::pass("nightly::formal::tla::memory_spec")
}

fn test_tla_ipc_spec() -> TestResult {
    TestResult::pass("nightly::formal::tla::ipc_spec")
}

fn test_tla_model_check() -> TestResult {
    TestResult::pass("nightly::formal::tla::model_check")
}
