pub mod kasan;
pub mod kmsan;
pub mod ubsan;
pub mod msan;
pub mod tsan;

use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    let mut tests = Vec::new();
    tests.extend(kasan::all_tests());
    tests.extend(kmsan::all_tests());
    tests.extend(ubsan::all_tests());
    tests.extend(msan::all_tests());
    tests.extend(tsan::all_tests());
    tests
}
