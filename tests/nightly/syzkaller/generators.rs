use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_syzkaller_generators_valid,
        &test_syzkaller_generators_diverse,
    ]
}

fn test_syzkaller_generators_valid() -> TestResult {
    TestResult::pass("nightly::syzkaller::generators::valid")
}

fn test_syzkaller_generators_diverse() -> TestResult {
    TestResult::pass("nightly::syzkaller::generators::diverse")
}
