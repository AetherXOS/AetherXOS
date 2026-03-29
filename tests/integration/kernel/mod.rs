pub mod boot;
pub mod memory;
pub mod scheduling;
pub mod ipc;
pub mod filesystem;
pub mod network;
pub mod drivers;
pub mod interrupts;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(boot::all_tests());
    tests.extend(memory::all_tests());
    tests.extend(scheduling::all_tests());
    tests.extend(ipc::all_tests());
    tests.extend(filesystem::all_tests());
    tests.extend(network::all_tests());
    tests.extend(drivers::all_tests());
    tests.extend(interrupts::all_tests());
    tests
}
