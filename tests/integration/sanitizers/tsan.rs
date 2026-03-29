use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_tsan_data_race,
        &test_tsan_lock_order,
    ]
}

fn test_tsan_data_race() -> TestResult {
    TestResult::pass("integration::sanitizers::tsan::data_race")
}

fn test_tsan_lock_order() -> TestResult {
    TestResult::pass("integration::sanitizers::tsan::lock_order")
}
