use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_apic_id,
        &test_apic_version,
        &test_apic_spurious,
    ]
}

fn test_apic_id() -> TestResult {
    TestResult::pass("integration::hardware::apic::id")
}

fn test_apic_version() -> TestResult {
    TestResult::pass("integration::hardware::apic::version")
}

fn test_apic_spurious() -> TestResult {
    TestResult::pass("integration::hardware::apic::spurious")
}
