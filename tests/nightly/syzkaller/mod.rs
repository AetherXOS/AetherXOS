pub mod descriptions;
pub mod syscalls;
pub mod generators;
pub mod corpus;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(descriptions::all_tests());
    tests.extend(syscalls::all_tests());
    tests.extend(generators::all_tests());
    tests.extend(corpus::all_tests());
    tests
}
