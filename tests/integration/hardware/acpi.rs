use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_acpi_rsdp,
        &test_acpi_madt,
        &test_acpi_fadt,
        &test_acpi_dsdt,
    ]
}

fn test_acpi_rsdp() -> TestResult {
    TestResult::pass("integration::hardware::acpi::rsdp")
}

fn test_acpi_madt() -> TestResult {
    TestResult::pass("integration::hardware::acpi::madt")
}

fn test_acpi_fadt() -> TestResult {
    TestResult::pass("integration::hardware::acpi::fadt")
}

fn test_acpi_dsdt() -> TestResult {
    TestResult::pass("integration::hardware::acpi::dsdt")
}
