use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_pci_config_space,
        &test_pci_device_enumeration,
        &test_pci_bar_mapping,
        &test_pci_msi,
    ]
}

fn test_pci_config_space() -> TestResult {
    TestResult::pass("integration::hardware::pci::config_space")
}

fn test_pci_device_enumeration() -> TestResult {
    TestResult::pass("integration::hardware::pci::device_enumeration")
}

fn test_pci_bar_mapping() -> TestResult {
    TestResult::pass("integration::hardware::pci::bar_mapping")
}

fn test_pci_msi() -> TestResult {
    TestResult::pass("integration::hardware::pci::msi")
}
