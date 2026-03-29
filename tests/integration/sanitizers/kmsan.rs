use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_kmsan_uninit_memory,
        &test_kmsan_uninit_copy,
    ]
}

fn test_kmsan_uninit_memory() -> TestResult {
    TestResult::pass("integration::sanitizers::kmsan::uninit_memory")
}

fn test_kmsan_uninit_copy() -> TestResult {
    TestResult::pass("integration::sanitizers::kmsan::uninit_copy")
}
