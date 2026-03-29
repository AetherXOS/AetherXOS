pub mod pci;
pub mod acpi;
pub mod apic;
pub mod ioapic;
pub mod hpet;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(pci::all_tests());
    tests.extend(acpi::all_tests());
    tests.extend(apic::all_tests());
    tests.extend(ioapic::all_tests());
    tests.extend(hpet::all_tests());
    tests
}
