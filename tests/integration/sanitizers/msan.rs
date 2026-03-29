use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_msan_uninit_read,
        &test_msan_uninit_write,
    ]
}

fn test_msan_uninit_read() -> TestResult {
    TestResult::pass("integration::sanitizers::msan::uninit_read")
}

fn test_msan_uninit_write() -> TestResult {
    TestResult::pass("integration::sanitizers::msan::uninit_write")
}
