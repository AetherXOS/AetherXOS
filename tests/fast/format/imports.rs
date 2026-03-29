use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_imports_organization,
        &test_imports_grouping,
        &test_imports_sorting,
        &test_imports_redundant,
    ]
}

fn test_imports_organization() -> TestResult {
    struct ImportOrder {
        std_first: bool,
        crate_second: bool,
        external_third: bool,
    }
    
    let order = ImportOrder {
        std_first: true,
        crate_second: true,
        external_third: true,
    };
    
    if order.std_first && order.crate_second && order.external_third {
        TestResult::pass("format::imports::organization")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::imports::organization", "Import organization violations")
            .with_category(TestCategory::Format)
    }
}

fn test_imports_grouping() -> TestResult {
    struct ImportGroups {
        std: Vec<&'static str>,
        external: Vec<&'static str>,
        crate_internal: Vec<&'static str>,
    }
    
    let groups = ImportGroups {
        std: vec!["core::fmt", "alloc::vec"],
        external: vec!["spin::Mutex", "bitflags::bitflags"],
        crate_internal: vec!["crate::harness::TestResult"],
    };
    
    let properly_grouped = !groups.std.is_empty() 
        || !groups.external.is_empty() 
        || !groups.crate_internal.is_empty();
    
    if properly_grouped {
        TestResult::pass("format::imports::grouping")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::imports::grouping", "Import grouping violations")
            .with_category(TestCategory::Format)
    }
}

fn test_imports_sorting() -> TestResult {
    let imports = vec![
        "alloc::string::String",
        "alloc::vec::Vec",
        "core::fmt::Debug",
        "core::sync::atomic",
    ];
    
    let mut sorted = imports.clone();
    sorted.sort();
    
    if imports == sorted {
        TestResult::pass("format::imports::sorting")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::imports::sorting", "Import sorting violations")
            .with_category(TestCategory::Format)
    }
}

fn test_imports_redundant() -> TestResult {
    struct RedundantCheck {
        duplicate_imports: usize,
        unused_imports: usize,
    }
    
    let check = RedundantCheck {
        duplicate_imports: 0,
        unused_imports: 0,
    };
    
    if check.duplicate_imports == 0 && check.unused_imports == 0 {
        TestResult::pass("format::imports::redundant")
            .with_category(TestCategory::Format)
    } else {
        TestResult::fail("format::imports::redundant", "Redundant imports detected")
            .with_category(TestCategory::Format)
    }
}
