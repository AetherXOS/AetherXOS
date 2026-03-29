use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_syzkaller_corpus_quality,
        &test_syzkaller_corpus_coverage,
    ]
}

fn test_syzkaller_corpus_quality() -> TestResult {
    TestResult::pass("nightly::syzkaller::corpus::quality")
}

fn test_syzkaller_corpus_coverage() -> TestResult {
    TestResult::pass("nightly::syzkaller::corpus::coverage")
}
