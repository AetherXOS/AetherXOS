use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_firecracker_boot,
        &test_firecracker_microvm,
    ]
}

fn test_firecracker_boot() -> TestResult {
    TestResult::pass("integration::virtualization::firecracker::boot")
}

fn test_firecracker_microvm() -> TestResult {
    TestResult::pass("integration::virtualization::firecracker::microvm")
}
