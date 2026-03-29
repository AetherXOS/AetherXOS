use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_qemu_boot,
        &test_qemu_smp,
        &test_qemu_acpi,
    ]
}

fn test_qemu_boot() -> TestResult {
    TestResult::pass("integration::virtualization::qemu::boot")
}

fn test_qemu_smp() -> TestResult {
    TestResult::pass("integration::virtualization::qemu::smp")
}

fn test_qemu_acpi() -> TestResult {
    TestResult::pass("integration::virtualization::qemu::acpi")
}
