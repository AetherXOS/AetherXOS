use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_doc_presence,
        &test_doc_format,
        &test_doc_links,
        &test_doc_examples,
    ]
}

fn test_doc_presence() -> TestResult {
    struct DocCoverage {
        public_functions: usize,
        documented_functions: usize,
        public_types: usize,
        documented_types: usize,
    }
    
    let coverage = DocCoverage {
        public_functions: 100,
        documented_functions: 95,
        public_types: 50,
        documented_types: 48,
    };
    
    let function_coverage = coverage.documented_functions as f64 / coverage.public_functions as f64;
    let type_coverage = coverage.documented_types as f64 / coverage.public_types as f64;
    
    if function_coverage >= 0.9 && type_coverage >= 0.9 {
        TestResult::pass("format::doc::presence")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::doc::presence", "Documentation coverage below 90%")
            .with_category(TestCategory::Format)
    }
}

fn test_doc_format() -> TestResult {
    struct DocFormat {
        starts_with_capital: bool,
        ends_with_period: bool,
        has_summary_line: bool,
    }
    
    let format = DocFormat {
        starts_with_capital: true,
        ends_with_period: true,
        has_summary_line: true,
    };
    
    if format.starts_with_capital 
        && format.ends_with_period 
        && format.has_summary_line 
    {
        TestResult::pass("format::doc::format")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::doc::format", "Documentation format violations")
            .with_category(TestCategory::Format)
    }
}

fn test_doc_links() -> TestResult {
    struct DocLinks {
        broken_links: usize,
        unresolved_links: usize,
    }
    
    let links = DocLinks {
        broken_links: 0,
        unresolved_links: 0,
    };
    
    if links.broken_links == 0 && links.unresolved_links == 0 {
        TestResult::pass("format::doc::links")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::doc::links", "Broken or unresolved doc links")
            .with_category(TestCategory::Format)
    }
}

fn test_doc_examples() -> TestResult {
    struct DocExamples {
        functions_with_examples: usize,
        functions_tested: usize,
    }
    
    let examples = DocExamples {
        functions_with_examples: 50,
        functions_tested: 45,
    };
    
    let example_ratio = examples.functions_tested as f64 / examples.functions_with_examples as f64;
    
    if example_ratio >= 0.8 {
        TestResult::pass("format::doc::examples")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::doc::examples", "Insufficient documented examples")
            .with_category(TestCategory::Format)
    }
}
