pub mod tla;
pub mod isabelle;
pub mod coq;
pub mod lean;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(tla::all_tests());
    tests.extend(isabelle::all_tests());
    tests.extend(coq::all_tests());
    tests.extend(lean::all_tests());
    tests
}
