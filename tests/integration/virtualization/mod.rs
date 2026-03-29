pub mod virtme;
pub mod qemu;
pub mod firecracker;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(virtme::all_tests());
    tests.extend(qemu::all_tests());
    tests.extend(firecracker::all_tests());
    tests
}
