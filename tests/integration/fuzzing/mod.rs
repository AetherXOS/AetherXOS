pub mod cargo_fuzz;
pub mod afl;
pub mod libfuzzer;
pub mod honggfuzz;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(cargo_fuzz::all_tests());
    tests.extend(afl::all_tests());
    tests.extend(libfuzzer::all_tests());
    tests.extend(honggfuzz::all_tests());
    tests
}
