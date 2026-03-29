pub mod flamegraph;
pub mod perf;
pub mod dhat;
pub mod massif;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(flamegraph::all_tests());
    tests.extend(perf::all_tests());
    tests.extend(dhat::all_tests());
    tests.extend(massif::all_tests());
    tests
}
