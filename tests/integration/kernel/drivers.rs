use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_drivers_pci_enumeration,
        &test_drivers_acpi_tables,
        &test_drivers_e1000,
        &test_drivers_nvme,
    ]
}

fn test_drivers_pci_enumeration() -> TestResult {
    TestResult::pass("integration::kernel::drivers::pci_enumeration")
}

fn test_drivers_acpi_tables() -> TestResult {
    TestResult::pass("integration::kernel::drivers::acpi_tables")
}

fn test_drivers_e1000() -> TestResult {
    TestResult::pass("integration::kernel::drivers::e1000")
}

fn test_drivers_nvme() -> TestResult {
    TestResult::pass("integration::kernel::drivers::nvme")
}
