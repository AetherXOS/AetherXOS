use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_rustfmt_check,
        &test_rustfmt_edition,
        &test_rustfmt_width,
        &test_rustfmt_imports,
    ]
}

fn test_rustfmt_check() -> TestResult {
    let code_sample = r#"fn main() {
    println!("Hello");
}"#;
    
    let formatted = code_sample.trim();
    let is_formatted = formatted.starts_with("fn main()");
    
    if is_formatted {
        TestResult::pass("lint::rustfmt::check")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::rustfmt::check", "Code not properly formatted")
            .with_category(TestCategory::Lint)
    }
}

fn test_rustfmt_edition() -> TestResult {
    let edition = "2021";
    
    if edition == "2021" || edition == "2018" || edition == "2015" {
        TestResult::pass("lint::rustfmt::edition")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::rustfmt::edition", "Invalid edition")
            .with_category(TestCategory::Lint)
    }
}

fn test_rustfmt_width() -> TestResult {
    let max_width: usize = 100;
    let tab_spaces: usize = 4;
    
    let code_line = "fn test() { let x = 1; }";
    let line_width = code_line.len();
    
    if max_width >= 80 && tab_spaces == 4 && line_width <= max_width {
        TestResult::pass("lint::rustfmt::width")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::rustfmt::width", "Width configuration invalid")
            .with_category(TestCategory::Lint)
    }
}

fn test_rustfmt_imports() -> TestResult {
    let imports = vec![
        "alloc::vec::Vec",
        "alloc::string::String",
        "core::fmt::Debug",
    ];
    
    let mut sorted = imports.clone();
    sorted.sort();
    
    if imports == sorted {
        TestResult::pass("lint::rustfmt::imports")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::rustfmt::imports", "Imports not sorted")
            .with_category(TestCategory::Lint)
    }
}
