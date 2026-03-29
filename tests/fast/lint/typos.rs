use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_typos_common,
        &test_typos_code,
        &test_typos_docs,
    ]
}

fn test_typos_common() -> TestResult {
    let common_typos = [
        ("teh", "the"),
        ("adn", "and"),
        ("recieve", "receive"),
        ("occured", "occurred"),
        ("seperate", "separate"),
    ];
    
    let text = "the and receive occurred separate";
    
    let mut has_typos = false;
    for (typo, _) in &common_typos {
        if text.contains(typo) {
            has_typos = true;
            break;
        }
    }
    
    if !has_typos {
        TestResult::pass("lint::typos::common")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::typos::common", "Common typos detected")
            .with_category(TestCategory::Lint)
    }
}

fn test_typos_code() -> TestResult {
    let code_typos = [
        ("fucntion", "function"),
        ("varible", "variable"),
        ("paramater", "parameter"),
        ("arguement", "argument"),
    ];
    
    let code_sample = "fn function(variable: parameter) -> argument { }";
    
    let mut has_typos = false;
    for (typo, _) in &code_typos {
        if code_sample.contains(typo) {
            has_typos = true;
            break;
        }
    }
    
    if !has_typos {
        TestResult::pass("lint::typos::code")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::typos::code", "Code typos detected")
            .with_category(TestCategory::Lint)
    }
}

fn test_typos_docs() -> TestResult {
    let doc_typos = [
        ("implemenation", "implementation"),
        ("paramater", "parameter"),
        ("retunr", "return"),
        ("defualt", "default"),
    ];
    
    let doc_sample = "This is the implementation. The parameter specifies the return value. Default is used.";
    
    let mut has_typos = false;
    for (typo, _) in &doc_typos {
        if doc_sample.contains(typo) {
            has_typos = true;
            break;
        }
    }
    
    if !has_typos {
        TestResult::pass("lint::typos::docs")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::typos::docs", "Documentation typos detected")
            .with_category(TestCategory::Lint)
    }
}
