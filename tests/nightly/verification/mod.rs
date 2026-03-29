pub mod kani;
pub mod prusti;
pub mod creusot;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(kani::all_tests());
    tests.extend(prusti::all_tests());
    tests.extend(creusot::all_tests());
    tests
}
