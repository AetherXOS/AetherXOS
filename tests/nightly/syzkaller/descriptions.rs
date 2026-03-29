use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_syzkaller_descriptions_syntax,
        &test_syzkaller_descriptions_coverage,
    ]
}

fn test_syzkaller_descriptions_syntax() -> TestResult {
    TestResult::pass("nightly::syzkaller::descriptions::syntax")
}

fn test_syzkaller_descriptions_coverage() -> TestResult {
    TestResult::pass("nightly::syzkaller::descriptions::coverage")
}
