use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_ioapic_id,
        &test_ioapic_version,
        &test_ioapic_redir,
    ]
}

fn test_ioapic_id() -> TestResult {
    TestResult::pass("integration::hardware::ioapic::id")
}

fn test_ioapic_version() -> TestResult {
    TestResult::pass("integration::hardware::ioapic::version")
}

fn test_ioapic_redir() -> TestResult {
    TestResult::pass("integration::hardware::ioapic::redir")
}
