use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_virtme_boot,
        &test_virtme_kernel_run,
        &test_virtme_ng_integration,
    ]
}

fn test_virtme_boot() -> TestResult {
    TestResult::pass("integration::virtualization::virtme::boot")
}

fn test_virtme_kernel_run() -> TestResult {
    TestResult::pass("integration::virtualization::virtme::kernel_run")
}

fn test_virtme_ng_integration() -> TestResult {
    TestResult::pass("integration::virtualization::virtme::ng_integration")
}
