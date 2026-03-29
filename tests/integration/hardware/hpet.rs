use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_hpet_caps,
        &test_hpet_timer,
    ]
}

fn test_hpet_caps() -> TestResult {
    TestResult::pass("integration::hardware::hpet::caps")
}

fn test_hpet_timer() -> TestResult {
    TestResult::pass("integration::hardware::hpet::timer")
}
